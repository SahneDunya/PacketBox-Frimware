#![no_std]

 use crate::firmware_common::{self, Error};

// Bellek haritası bilgileri - Doğrulayın!
pub const LPDDR1_BASE_ADDRESS: usize = 0x8000_0000; // Örnek adres
pub const LPDDR1_SIZE_BYTES: usize = 2 * 1024 * 1024; // 2 MB
pub const LPDDR1_END_ADDRESS: usize = LPDDR1_BASE_ADDRESS + LPDDR1_SIZE_BYTES;

#[derive(Debug)]
pub enum MemoryError {
    InitializationError,
    // ...
}

/// LPDDR1 belleği başlatır.
/// # Safety
/// Donanım yazmaçlarına doğrudan erişim gerektirir, "unsafe"dir.
pub unsafe fn init_memory() -> Result<(), MemoryError> {
    // --- GERÇEK BAŞLATMA KODU BURAYA GELECEK ---
    // SiFive S21 Bellek Kontrolcüsünü (Memory Controller) LPDDR1 spesifikasyonlarına göre yapılandırın.
    // Bu adım çok karmaşıktır ve detaylı LPDDR1 ve Bellek Kontrolcüsü belgeleri gerektirir.

    // Örnek placeholder: Bellek Kontrolcüsü Register Ayarları ve Başlatma Dizisi
    // Buradaki adresler ve değerler tamamen varsayımsaldir!
     const MEM_CTRL_BASE: usize = 0xXXXX_0000; // SiFive S21 Memory Controller Base Adresi - VERİ SAYFASINDAN BULUN!
     const MEM_CTRL_CFG0: usize = 0x00; // Yapılandırma Register Ofseti - VERİ SAYFASINDAN BULUN!
     const MEM_CTRL_TIMING0: usize = 0x10; // Zamanlama Register Ofseti - VERİ SAYFASINDAN BULUN!
     const MEM_CTRL_COMMAND: usize = 0x40; // Komut Register Ofseti - VERİ SAYFASINDAN BULUN!
     const MEM_CTRL_STATUS: usize = 0x50; // Durum Register Ofseti - VERİ SAYFASINDAN BULUN!

     let cfg0_ptr = (MEM_CTRL_BASE + MEM_CTRL_CFG0) as *mut u32;
     let timing0_ptr = (MEM_CTRL_BASE + MEM_CTRL_TIMING0) as *mut u32;
     let command_ptr = (MEM_CTRL_BASE + MEM_CTRL_COMMAND) as *mut u32;
     let status_ptr = (MEM_CTRL_BASE + MEM_CTRL_STATUS) as *mut u32;


    // // 1. Kontrolcüyü Temel Modda Yapılandırma
     cfg0_ptr.write_volatile(0x12345678); // Varsayımsal yapılandırma değeri (veri yolu genişliği, tip vb.)

    // // 2. LPDDR1 Zamanlama Parametrelerini Ayarlama (tCAS, tRP, tRAS, tRFC vb.)
     timing0_ptr.write_volatile(0xABCDEF01); // Varsayımsal zamanlama değeri

    // // 3. LPDDR1 Başlatma Dizisini Gerçekleştirme (Power-up Sequence)
    // // Bu, LPDDR1 standardına göre belirli komutları (NOP, Precharge All, Auto Refresh, Mode Register ayarları)
    // // belirli gecikmelerle bellek kontrolcüsü aracılığıyla göndermeyi içerir.
     command_ptr.write_volatile(0x01); // Örnek: NOP komutu gönder
     delay_microseconds(10); // Varsayımsal gecikme
     command_ptr.write_volatile(0x02); // Örnek: Precharge All komutu gönder
     delay_microseconds(20); // Varsayımsal gecikme
    // // ... dizinin geri kalanı ...

    // // 4. Başlatmanın Tamamlanmasını Bekleme (Durum Register'ından)
     while (status_ptr.read_volatile() & (1 << 0)) == 0 { /* Bekle */ } // Örnek: Hazır bitini bekle

    // // Bellek testleri yapılabilir (isteğe bağlı)
     check_memory_range(LPDDR1_BASE_ADDRESS, LPDDR1_SIZE_BYTES)?;


    // Başlatma başarılı olursa
    Ok(())

    // Başlatma sırasında hata oluşursa (örneğin, durum register'ından okunabilir)
     Err(MemoryError::InitializationError)
}

extern crate alloc;
use linked_list_allocator::LockedHeap;
static mut HEAP: [u8; LPDDR1_SIZE_BYTES] = [0; LPDDR1_SIZE_BYTES];
#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();
pub unsafe fn init_allocator() -> Result<(), MemoryError> {
    ALLOCATOR.lock().init(HEAP.as_mut_ptr(), HEAP.len());
    Ok(())
}
