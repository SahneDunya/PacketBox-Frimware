#![no_std]

use core::fmt::Write;

// SiFive S21 UART0 Base Adresi - VERİ SAYFASINDAN BULUN!
pub const UART0_BASE_ADDRESS: usize = 0x1000_0000; // Örnek adres - Doğrulayın!

// UART0 register ofsetleri - VERİ SAYFASINDAN BULUN!
pub const UART_TXDATA: usize = 0x00;
pub const UART_RXDATA: usize = 0x04;
pub const UART_TXCTRL: usize = 0x08;
pub const UART_RXCTRL: usize = 0x0C;
pub const UART_IE: usize = 0x10; // Interrupt Enable
pub const UART_IP: usize = 0x14; // Interrupt Pending
pub const UART_SCALECFG: usize = 0x18; // Baud Rate Divisor

// UART kontrol bitleri (TXCTRL) - VERİ SAYFASINDAN BULUN!
pub const UART_TXEN: u32 = 1 << 0; // Transmit Enable
pub const UART_RXEN: u32 = 1 << 0; // Receive Enable (RXCTRL'de)


// Baud Rate Bölücü Yapılandırma Değeri - SİSTEM SAATİNE GÖRE HESAPLAYIN!
// SCALECFG = (Clock Frequency / Baud Rate) - 1
// Örnek: 10 MHz saat ve 115200 baud rate için ~86
pub const UART_BAUD_DIVISOR: u32 = 86; // Örnek değer - Doğrulayın!


pub struct Uart0 {
    base_address: usize,
}

impl Uart0 {
    pub const fn new(base_address: usize) -> Self {
        Uart0 { base_address }
    }

    /// UART0 donanımını başlatır.
    /// # Safety
    /// Donanım yazmaçlarına doğrudan eriştiği için "unsafe"dir.
    pub unsafe fn init(&mut self) { // Result<(), Error> döndürebilir
        // --- GERÇEK BAŞLATMA KODU BURAYA GELECEK ---
        // SiFive S21 UART0 çevre birimini yapılandırın.

        let txctrl_ptr = (self.base_address + UART_TXCTRL) as *mut u32;
        let rxctrl_ptr = (self.base_address + UART_RXCTRL) as *mut u32;
        let scalecfg_ptr = (self.base_address + UART_SCALECFG) as *mut u32;

        // 1. Baud rate bölücüyü ayarla.
        // BU DEĞER KULLANDIĞINIZ SİSTEM SAATİNE VE İSTEDİĞİNİZ BAUD RATE'E GÖRE HESAPLANMALIDIR!
        scalecfg_ptr.write_volatile(UART_BAUD_DIVISOR);

        // 2. İletim (TX) ve Alma (RX) birimlerini etkinleştir.
        // TXCTRL register'ının formatını SiFive belgelerinden kontrol edin.
        let mut tx_ctrl_val = txctrl_ptr.read_volatile();
        tx_ctrl_val |= UART_TXEN; // TX etkinleştir
        txctrl_ptr.write_volatile(tx_ctrl_val);

        // RXCTRL register'ının formatını SiFive belgelerinden kontrol edin.
        let mut rx_ctrl_val = rxctrl_ptr.read_volatile();
        rx_ctrl_val |= UART_RXEN; // RX etkinleştir
        rxctrl_ptr.write_volatile(rx_ctrl_val);


        // 3. Diğer ayarlar (Data bits, Parity, Stop bits)
        // Bunlar genellikle TXCTRL/RXCTRL yazmaçlarının diğer bitleriyle ayarlanır.
        // SiFive belgelerinden bu yazmaçların formatını kontrol edin ve gerekirse ayarlayın.
         tx_ctrl_val |= (1 << 1); // Varsayımsal 9 data bit ayari
         txctrl_ptr.write_volatile(tx_ctrl_val);
    }

    /// UART'a bir bayt gönderir. İletim tamponu doluysa bekler (blocking).
    /// # Safety
    /// Donanım yazmacına doğrudan yazıldığı için "unsafe"dir.
    pub unsafe fn putc(&mut self, byte: u8) {
        let txdata_ptr = (self.base_address + UART_TXDATA) as *mut u32;

        // TXDATA yazmacının dolu (full) olup olmadığını kontrol et.
        // SiFive UART'ta TXDATA'nın bit 31'i dolu ise 1'dir. Boş olana kadar bekle.
         while (txdata_ptr.read_volatile() & (1 << 31)) != 0 {
             // Bekle
         }

        // Veriyi gönder (alt 8 bit kullanılır).
        txdata_ptr.write_volatile(byte as u32);
    }

    /// UART'tan bir bayt okur, tampon boşsa hemen None döner (non-blocking).
    /// # Safety
    /// Donanım yazmacından doğrudan okunduğu için "unsafe"dir.
    pub unsafe fn read_byte(&mut self) -> Option<u8> {
        let rxdata_ptr = (self.base_address + UART_RXDATA) as *mut u32;

        // RXDATA yazmacının boş (empty) olup olmadığını kontrol et (bit 31).
        if (rxdata_ptr.read_volatile() & (1 << 31)) == 0 {
            // Veri var, oku (okumak register'daki veriyi temizler) ve Some(byte) olarak döndür.
            Some((rxdata_ptr.read_volatile() & 0xFF) as u8)
        } else {
            // Tampon boş, None döndür.
            None
        }
    }

    /// UART'tan bir bayt okur. Alma tamponu boşsa bekler (blocking).
    /// # Safety
    /// Donanım yazmacından doğrudan okunduğu için "unsafe"dir.
     pub unsafe fn getc(&mut self) -> u8 {
        let rxdata_ptr = (self.base_address + UART_RXDATA) as *mut u32;

        // RXDATA yazmacının boş (empty) olup olmadığını kontrol et (bit 31).
        while (rxdata_ptr.read_volatile() & (1 << 31)) != 0 {
            // Bekle
        }

        // Veriyi oku (okumak register'daki veriyi temizler) ve alt 8 bitini döndür.
        (rxdata_ptr.read_volatile() & 0xFF) as u8
    }
}

impl Write for Uart0 {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            for byte in s.as_bytes() {
                self.putc(*byte);
            }
        }
        Ok(())
    }
}

pub static mut UART0_GLOBAL: Uart0 = Uart0::new(UART0_BASE_ADDRESS);