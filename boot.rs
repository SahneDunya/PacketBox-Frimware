#![no_std]

use crate::memory::MemoryError;
use crate::uart::Uart0; // initialize_peripherals icinde UART init cagrisi icin (veya global UART)

#[derive(Debug)]
pub enum BootError {
    EarlyHardwareInitError,
    MemoryInitError(MemoryError),
    PeripheralInitError,
}

impl From<MemoryError> for BootError {
    fn from(err: MemoryError) -> Self {
        BootError::MemoryInitError(err)
    }
}


/// Sistemdeki en erken donanım başlatma adımlarını gerçekleştirir.
/// Bu, genellikle saat kaynaklarını ve temel güç ayarlarını yapılandırmayı içerir.
/// # Safety
/// Donanım yazmaçlarına doğrudan erişim gerektirir, "unsafe"dir.
pub unsafe fn perform_early_hardware_init() -> Result<(), BootError> {
    // --- GERÇEK BAŞLATMA KODU BURAYA GELECEK ---
    // SiFive S21'in Clock Generator ve Power Management birimlerini yapılandırın.

    // Örnek placeholder: Sistem Saat Kaynağını ve PLL'leri Yapılandırma
    // İşlemcinin ve çevre birimlerinin doğru frekansta çalışması için kritik.
    // Buradaki adresler ve değerler tamamen varsayımsaldir!
     const CLOCK_GEN_BASE: usize = 0xXXXX_0000; // SiFive S21 Clock Generator Base Adresi - VERİ SAYFASINDAN BULUN!
     const CLOCK_PLL_CFG: usize = 0x04; // PLL Yapılandırma Register Ofseti - VERİ SAYFASINDAN BULUN!
     const CLOCK_ENABLE_REG: usize = 0x08; // Saat Etkinleştirme Register Ofseti - VERİ SAYFASINDAN BULUN!

     let pll_cfg_ptr = (CLOCK_GEN_BASE + CLOCK_PLL_CFG) as *mut u32;
     let enable_reg_ptr = (CLOCK_GEN_BASE + CLOCK_ENABLE_REG) as *mut u32;

    // // Örnek: PLL'i yapılandır (Değerler frekans hesabına göre değişir)
     pll_cfg_ptr.write_volatile(0x12345678); // Varsayımsal PLL yapılandırma değeri

    // // Örnek: Ana saatleri etkinleştir
     enable_reg_ptr.write_volatile(0xFF); // Varsayımsal etkinleştirme değeri

    // Örnek placeholder: Temel Güç Yönetimi Ayarları
    // Gerekirse voltaj regülatörlerini ayarlama veya düşük güç modlarını yapılandırma.
     const POWER_MGMT_BASE: usize = 0xYYYY_0000; // SiFive S21 Power Management Base Adresi - VERİ SAYFASINDAN BULUN!
    // ... register erişimleri ...


    // Başlatma başarılı olursa
    Ok(())

    // Başlatma sırasında hata oluşursa (donanım register'larından okunabilir)
     Err(BootError::EarlyHardwareInitError)
}

/// Bellek sistemini başlatır (LPDDR1).
/// # Safety
/// Bellek kontrolcüsüne erişim içerebilir, bu nedenle "unsafe"dir.
pub unsafe fn initialize_memory() -> Result<(), BootError> {
    // --- GERÇEK BAŞLATMA KODU BURAYA GELECEK ---
    // `memory.rs` dosyasındaki LPDDR1 başlatma fonksiyonunu çağır.
    // Bu fonksiyonun içi artık gerçek başlatma adımlarını içermelidir.
    crate::memory::init_memory().map_err(BootError::MemoryInitError)
}

/// Temel çevre birimlerini başlatır (UART gibi).
/// # Safety
/// Çevre birimi donanımına erişim içerebilir, bu nedenle "unsafe"dir.
pub unsafe fn initialize_peripherals() -> Result<(), BootError> {
    // --- GERÇEK BAŞLATMA KODU BURAYA GELECEK ---
    // UART0 gibi temel çevre birimlerini başlat.
    // UART başlatma fonksiyonu artık gerçek register ayarlamalarını içermelidir.
    // UART'ın init fonksiyonu Result döndürmediği için başarı varsayılıyor:
    crate::uart::UART0_GLOBAL.init();
    // Eğer UART init Result döndürseydi:
    // crate::uart::UART0_GLOBAL.init().map_err(|_| BootError::PeripheralInitError)?;

    // Diğer temel çevre birimleri (örneğin, basit bir Timer veya Watchdog Timer)
    // ... timer_module::init()?; ...


    // Başarılı sayalım
    Ok(())
}