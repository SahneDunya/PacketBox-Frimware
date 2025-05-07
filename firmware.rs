#![no_std]
#![no_main]

use riscv_rt::entry;
use core::fmt::Write; // writeln! makrosu için

// Modüllerin içe aktarılması
mod firmware_common;
mod uart;
mod memory;
mod storage;
mod psu;
mod refrigerator;
mod cli;
mod boot; // Boot aşamaları

// Ortak öğeler ve genel hata türü
use crate::firmware_common::{self, log, Error, SystemState, set_system_state};

// Global staticlere erişim için use bildirimleri
use crate::uart::UART0_GLOBAL;
use crate::cli::CLI_GLOBAL;
use crate::storage::{EMIC_STORAGE_GLOBAL, SD_CARD_STORAGE_GLOBAL}; // Storage global statikleri
use crate::psu::PSU_MONITOR_GLOBAL; // PSU global statik
use crate::refrigerator::FRIDGE_CONTROLLER_GLOBAL; // Refrigerator global statik


// Firmware'in ana giriş noktası.
#[entry]
fn main() -> ! {
    // İşlemci riscv-rt tarafindan baslatildi. Yigin, BSS/Data hazir.
    // Donanıma özgü başlatma aşamalarını yönetelim.

    unsafe { set_system_state(SystemState::Initializing); }
     log!("System State: Initializing"); // Bu log UART init'ten once calismaz!

    // --- Boot Süreci Aşamaları ---

    // 1. Erken Donanım Başlatma (Saat, Temel Güç)
    //    UART çalışmadan önce olabilecek hatalar için özel raporlama gerekebilir (LED gibi).
    match unsafe { boot::perform_early_hardware_init() } {
        Ok(_) => { /* Başarılı */ },
        Err(_) => unsafe {
             // Hata! UART yok, LED yakarak veya baska bir yolla raporla
             set_system_state(SystemState::Error);
             loop {} // Kurtarılamaz hata
        }
    }

    // 2. Temel Çevre Birimleri Başlatma (UART - loglama için)
    //    UART başlatıldıktan sonra log! makrosu kullanılabilir.
    match unsafe { boot::initialize_peripherals() } {
        Ok(_) => {
            // UART artık çalışıyor olmalı. log! makrosu kullanılabilir.
            unsafe { log!("UART and core peripherals initialized.") };
        },
        Err(e) => unsafe {
            // UART başlatma hatası. Loglama mümkün değil.
            // Belki ham putc deneyebiliriz veya LED yakabiliriz.
             unsafe { UART0_GLOBAL.putc(b'U'); UART0_GLOBAL.putc(b'E'); }
             set_system_state(SystemState::Error);
             loop {} // Kurtarılamaz hata
        }
    }

    // 3. Bellek Başlatma (LPDDR1)
    match unsafe { boot::initialize_memory() } {
        Ok(_) => unsafe { log!("Memory initialized successfully.") },
        Err(e) => unsafe {
            log!("Memory initialization FAILED: {:?}", e);
            set_system_state(SystemState::Error);
            loop {} // Kurtarılamaz hata
        }
    }

    // İsteğe Bağlı: Global Bellek Ayırıcıyı Başlatma (Eğer kullanılıyorsa)
    
    unsafe {
        match crate::memory::init_allocator() {
             Ok(_) => log!("Global allocator initialized."),
             Err(e) => {
                 log!("Global allocator initialization FAILED: {:?}", e);
                 set_system_state(SystemState::Error);
                 loop {} // Kurtarılamaz hata
             }
        }
    }
    


    // --- Diğer Sistem Modüllerini Başlatma ---

    // Depolama Başlatma ve Global Statiklere Atama
    unsafe {
        match crate::storage::init_storage_devices() { // Parametreler burada paslanmalı
             Ok((emic_opt, sd_opt)) => {
                 EMIC_STORAGE_GLOBAL = emic_opt;
                 SD_CARD_STORAGE_GLOBAL = sd_opt;

                 if EMIC_STORAGE_GLOBAL.is_some() { log!("eMMC storage initialized.") }
                 if SD_CARD_GLOBAL.is_some() { log!("SD card storage initialized.") }
                 if EMIC_STORAGE_GLOBAL.is_none() && SD_CARD_GLOBAL.is_none() { log!("No storage devices initialized.") }
             },
             Err(e) => {
                 log!("Storage initialization FAILED: {:?}", e);
                 // Depolama hatası kurtarılamaz olmayabilir, duruma göre karar verin.
                  set_system_state(SystemState::Error); loop {};
             }
        }
    }

    // PSU İzleyici Başlatma ve Global Statiğe Atama
     unsafe {
         let mut psu_monitor = crate::psu::PsuMonitor::new(); // Parametreler burada paslanmalı
        match psu_monitor.init() {
             Ok(_) => {
                PSU_MONITOR_GLOBAL = Some(psu_monitor);
                 log!("PSU monitor initialized.");
                  // PSU'yu aç (eğer soft-power kontrolü varsa)
                   unsafe { if let Some(psu) = &mut PSU_MONITOR_GLOBAL { let _ = psu.turn_on(); } }
                  // Power Good sinyalini bekle (boot.rs'de veya burada yapılabilir)
                   match boot::wait_for_power_good() { ... }
             },
             Err(e) => {
                 log!("PSU monitor initialization FAILED: {:?}", e);
                 // PSU hatası kurtarılamaz olmayabilir.
                  set_system_state(SystemState::Error); loop {};
             }
        }
     }

    // Buzdolabı Arayüzü Başlatma ve Global Statiğe Atama
     unsafe {
        let mut fridge_controller = crate::refrigerator::RefrigeratorController::new(); // Parametreler burada paslanmalı
        match fridge_controller.init() {
             Ok(_) => {
                FRIDGE_CONTROLLER_GLOBAL = Some(fridge_controller);
                 log!("Refrigerator interface initialized.");
             },
             Err(e) => {
                 log!("Refrigerator interface initialization FAILED: {:?}", e);
                 // Buzdolabı hatası kurtarılamaz olmayabilir.
                  set_system_state(SystemState::Error); loop {};
             }
        }
     }


    // CLI Başlatma (UART'a bağımlı olduğu için diğer çevre birimlerinden sonra)
     unsafe {
        CLI_GLOBAL.init(); // CLI state'ini sıfırla
        let _ = CLI_GLOBAL.print_prompt(&mut UART0_GLOBAL); // İlk prompt'u yazdır
        log!("CLI initialized. Type 'help'.");
     }


    // --- Başlatma Tamamlandı ---
    unsafe { set_system_state(SystemState::Running); }
    unsafe { log!("PacketBox System is now Running."); }


    // --- Ana Çalışma Döngüsü (BIOS Benzeri CLI Etkileşimi) ---
    // Bu döngüde sistem CLI input bekler ve diğer temel görevleri (varsa) yapar.
    loop {
        // 1. CLI Girişini İşleme (Polling)
        // UART'tan byte gelip gelmediğini non-blocking olarak kontrol et.
        // Eğer byte geldiyse CLI işleyicisine ilet.
        unsafe {
            if let Some(byte) = UART0_GLOBAL.read_byte() {
                 // Byte geldiyse CLI'ya işle. process_byte kendi içinde echo yapar ve komut çalıştırır.
                 let _ = CLI_GLOBAL.process_byte(byte, &mut UART0_GLOBAL);
            }
        }

        // 2. Diğer Periyodik veya Olay Tabanlı Görevler
        // CLI input beklenmediği zaman CPU burayı çalıştırır.
        // - Buzdolabı durumu kontrolü
        // - PSU durumu izleme
        // - Sistem sağlığı kontrolü
        // - Çok kısa beklemeler (CPU'yu tamamen meşgul etmemek için, polling yapılıyorsa)
        // crate::firmware_common::delay_cycles(100); // Varsayımsal kısa bekleme

        // Eğer interrupt kullanılıyorsa, burada WFI (Wait For Interrupt) kullanılabilir:
         riscv::asm::wfi(); // interrupt gelene kadar bekler

    }
}

// Hata işleyici fonksiyonu (Panic handler)
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Panic bilgilerini UART üzerinden raporla.
    unsafe {
        let mut uart = &mut UART0_GLOBAL; // Global UART'a unsafe erişim
        let _ = writeln!(uart, "\n--- PANIC ---"); // Yeni satir ekleyerek CLI promptunu bozmamaya calis
        if let Some(location) = info.location() {
            let _ = writeln!(uart, "at {}:{}", location.file(), location.line());
        } else {
            let _ = writeln!(uart, "at unknown location");
        }
        if let Some(message) = info.payload().downcast_ref::<&'static str>() {
             let _ = writeln!(uart, "Message: {}", message);
        } else {
             let _ = writeln!(uart, "Message: <unknown>");
        }
        let _ = writeln!(uart, "--- PANIC ---");
    }

    unsafe { set_system_state(SystemState::Error); }

    // Kurtarılamaz hata durumunda sonsuz döngüye gir.
    loop { /* Hata ledi yak vb. */ }
}

// Varsayılan İstisna (Exception) İşleyici (isteğe bağlı, riscv-rt sağlar)

use riscv_rt::TrapFrame;

#[exception]
unsafe fn DefaultExceptionHandler(trapframe: &TrapFrame) -> ! {
     let mut uart = &mut UART0_GLOBAL;
     let _ = writeln!(uart, "\n--- EXCEPTION ---"); // Yeni satir ekle
     // mcause degerini daha anlamli yazdiran bir fonksiyon kullanin
     let _ = writeln!(uart, "Cause: {}", trapframe.mcause());
     let _ = writeln!(uart, "PC: {:#x}", trapframe.mepc());
     let _ = writeln!(uart, "--- EXCEPTION ---");

    unsafe { set_system_state(SystemState::Error); }
    loop { /* Hata ledi yak */ }
}