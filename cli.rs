#![no_std]

use core::fmt::Write;
use crate::uart::Uart0; // UART0_GLOBAL'i kullanmak için

// Diğer modüllerdeki global statiklere erişim için use bildirimleri
use crate::storage::{EMIC_STORAGE_GLOBAL, SD_CARD_STORAGE_GLOBAL, BlockDevice}; // Global storage
use crate::memory::{LPDDR1_SIZE_BYTES /*, ALLOCATOR */}; // Global memory bilgisi/allocator
use crate::psu::PSU_MONITOR_GLOBAL; // Global PSU
use crate::refrigerator::FRIDGE_CONTROLLER_GLOBAL; // Global Buzdolabi

// Maximum size of the input buffer for a single command line.
const INPUT_BUFFER_SIZE: usize = 128;
// Maximum number of arguments a command can have.
const MAX_ARGS: usize = 10;
// The prompt string displayed by the CLI.
const PROMPT: &str = "> ";

// Errors specific to the CLI module.
#[derive(Debug)]
pub enum CliError {
    BufferFull,
    UnknownCommand,
    TooManyArguments,
    InvalidDataFormat, // UTF-8 decode hatasi gibi
    CommandFailed,
    UartWriteError,
    // Diğer modullerden sarilmis hatalar (istege bagli, firmware_common::Error zaten var)
     Storage(crate::storage::StorageError),
    // ...
}

// Represents a registered CLI command.
struct Command {
    name: &'static str,
    help: &'static str,
    // Komut fonksiyonu: UART yazıcısını ve argüman dilimini alır.
    execute: fn(&mut Uart0, &[&str]) -> Result<(), CliError>,
}

// The main CLI state structure.
pub struct Cli {
    buffer: [u8; INPUT_BUFFER_SIZE],
    index: usize,
}

impl Cli {
    pub const fn new() -> Self {
        Cli { buffer: [0u8; INPUT_BUFFER_SIZE], index: 0 }
    }

    pub fn init(&mut self) {
        self.index = 0;
    }

    /// Gelen baytı işler.
    /// # Safety
    /// Global UART'a yazma gerektirir.
    pub unsafe fn process_byte(&mut self, byte: u8, uart: &mut Uart0) -> Result<(), CliError> {
        match byte {
            0x08 | 0x7F => { // Backspace veya Delete
                if self.index > 0 {
                    self.index -= 1;
                    uart.write_str("\x08 \x08").map_err(|_| CliError::UartWriteError)?;
                }
            }
            0x0D => { // Carriage Return
                uart.write_str("\r\n").map_err(|_| CliError::UartWriteError)?;
                self.parse_and_execute(uart)?;
                self.index = 0;
                self.print_prompt(uart)?;
            }
            0x0A => { // Newline
                // CR'den sonra geldiyse yoksay veya CR'den sonra gelmediyse işle
                 if self.index > 0 && self.buffer[self.index - 1] != 0x0D {
                     uart.write_str("\r\n").map_err(|_| CliError::UartWriteError)?;
                     self.parse_and_execute(uart)?;
                     self.index = 0;
                     self.print_prompt(uart)?;
                 } else if self.index == 0 {
                     self.print_prompt(uart)?;
                 }
            }
            _ if byte < 0x20 => { /* Kontrol karakterlerini yoksay */ }
            _ => { // Yazdırılabilir karakter
                if self.index < INPUT_BUFFER_SIZE {
                    self.buffer[self.index] = byte;
                    self.index += 1;
                    uart.write_str(core::str::from_utf8(core::slice::from_ref(&byte)).map_err(|_| CliError::InvalidDataFormat)?).map_err(|_| CliError::UartWriteError)?;
                } else {
                    uart.write_str("\x07").map_err(|_| CliError::UartWriteError)?; // BEL
                    return Err(CliError::BufferFull);
                }
            }
        }
        Ok(())
    }

    /// Arabellekteki komutu ayrıştırıp çalıştırır.
    /// # Safety
    /// Komut fonksiyonları global statiklere erişebilir. UART'a yazma gerektirir.
    unsafe fn parse_and_execute(&mut self, uart: &mut Uart0) -> Result<(), CliError> {
        if self.index == 0 { return Ok(()); }

        let line_bytes = &self.buffer[..self.index];
        let line_str = core::str::from_utf8(line_bytes).map_err(|_| CliError::InvalidDataFormat)?;
        let line_str = line_str.trim();

        if line_str.is_empty() { return Ok(()); }

        let mut parts = line_str.split_whitespace();
        let command_name = parts.next().unwrap();
        let mut args: [&str; MAX_ARGS] = [""; MAX_ARGS];
        let mut arg_count = 0;
        for arg in parts {
            if arg_count < MAX_ARGS { args[arg_count] = arg; arg_count += 1; }
            else { writeln!(uart, "Error: Too many arguments.").map_err(|_| CliError::UartWriteError)?; return Err(CliError::TooManyArguments); }
        }
        let args_slice = &args[..arg_count];

        let commands = get_commands();
        for command in commands {
            if command.name == command_name {
                return (command.execute)(uart, args_slice); // Komutu çalıştır
            }
        }

        writeln!(uart, "Error: Unknown command '{}'. Type 'help'.", command_name).map_err(|_| CliError::UartWriteError)?;
        Err(CliError::UnknownCommand)
    }

    /// Prompt yazdırır.
    /// # Safety
    /// Global UART'a yazma gerektirir.
    unsafe fn print_prompt(&self, uart: &mut Uart0) -> Result<(), CliError> {
        uart.write_str(PROMPT).map_err(|_| CliError::UartWriteError)
    }
}

// --- Komut Implementasyonları ---

fn get_commands() -> &'static [Command] {
    &[
        Command { name: "help", help: "Show help.", execute: help_command },
        Command { name: "status", help: "Show system status.", execute: status_command },
        Command { name: "storage", help: "Interact with storage (read/write/info). Usage: storage info <emic|sd>", execute: storage_command },
        Command { name: "boot", help: "Attempt to boot from a device.", execute: boot_command },
        // Diğer komutlar buraya eklenecek
         Command { name: "fridge", help: "Control refrigerator.", execute: fridge_command },
         Command { name: "psu", help: "PSU status.", execute: psu_command },
    ]
}

// help komutu (Zaten hazırdı)
unsafe fn help_command(uart: &mut Uart0, args: &[&str]) -> Result<(), CliError> {
     let commands = get_commands();
    if args.is_empty() {
        writeln!(uart, "Available commands:").map_err(|_| CliError::UartWriteError)?;
        for cmd in commands {
            writeln!(uart, "  {}: {}", cmd.name, cmd.help).map_err(|_| CliError::UartWriteError)?;
        }
    } else {
        let target_command = args[0];
        let mut found = false;
        for cmd in commands {
            if cmd.name == target_command {
                writeln!(uart, "{}: {}", cmd.name, cmd.help).map_err(|_| CliError::UartWriteError)?;
                found = true;
                break;
            }
        }
        if !found {
             writeln!(uart, "Error: Command '{}' not found.", target_command).map_err(|_| CliError::UartWriteError)?;
        }
    }
    Ok(())
}

// status komutu (Global statiklere erişim eklendi)
unsafe fn status_command(uart: &mut Uart0, _args: &[&str]) -> Result<(), CliError> {
    writeln!(uart, "PacketBox System Status:").map_err(|_| CliError::UartWriteError)?;
    writeln!(uart, "  Firmware: PacketBox v{} ({})", "1.0", "BuildDate").map_err(|_| CliError::UartWriteError)?; // Versiyon/BuildDate sabitleri eklenebilir
    writeln!(uart, "  State: {:?}", firmware_common::get_system_state()).map_err(|_| CliError::UartWriteError)?;
    writeln!(uart, "  Memory: {} Bytes LPDDR1", LPDDR1_SIZE_BYTES).map_err(|_| CliError::UartWriteError)?;

    // Global Storage durumunu raporla
    writeln!(uart, "  Storage:").map_err(|_| CliError::UartWriteError)?;
    let emic_status = if EMIC_STORAGE_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
    let emic_blocks = if let Some(emic) = &mut EMIC_STORAGE_GLOBAL { emic.block_count().unwrap_or(0) } else { 0 }; // unsafe erişim
    writeln!(uart, "    eMMC 1.0: {} ({} blocks)", emic_status, emic_blocks).map_err(|_| CliError::UartWriteError)?;

    let sd_status = if SD_CARD_STORAGE_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
    let sd_blocks = if let Some(sd) = &mut SD_CARD_STORAGE_GLOBAL { sd.block_count().unwrap_or(0) } else { 0 }; // unsafe erişim
    writeln!(uart, "    SD Card: {} ({} blocks)", sd_status, sd_blocks).map_err(|_| CliError::UartWriteError)?;

    // Global PSU durumunu raporla
    writeln!(uart, "  PSU:").map_err(|_| CliError::UartWriteError)?;
    let psu_status = if PSU_MONITOR_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
    // Eğer is_power_good fonksiyonu varsa kullanılabilir:
     let power_good = if let Some(psu) = &mut PSU_MONITOR_GLOBAL { psu.is_power_good().unwrap_or(false) } else { false };
     writeln!(uart, "    Monitor: {} (Power Good: {})", psu_status, power_good).map_err(|_| CliError::UartWriteError)?;
    writeln!(uart, "    Monitor: {}", psu_status).map_err(|_| CliError::UartWriteError)?;


    // Global Refrigerator durumunu raporla
    writeln!(uart, "  Refrigerator:").map_err(|_| CliError::UartWriteError)?;
    let fridge_status = if FRIDGE_CONTROLLER_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
    writeln!(uart, "    Controller: {}", fridge_status).map_err(|_| CliError::UartWriteError)?;


    Ok(())
}

// storage komutu (Sadece info kısmı implement edildi)
unsafe fn storage_command(uart: &mut Uart0, args: &[&str]) -> Result<(), CliError> {
    if args.len() < 2 || args[0] != "info" {
        writeln!(uart, "Usage: storage info <emic|sd>").map_err(|_| CliError::UartWriteError)?;
        // read/write implementasyonlari buraya eklenecek
         writeln!(uart, "Usage: storage <emic|sd> <read|write> <lba> [data_byte]").map_err(|_| CliError::UartWriteError)?;
        return Ok(());
    }

    let device_type = args[1];

    match device_type {
        "emic" => {
            writeln!(uart, "eMMC 1.0 Info:").map_err(|_| CliError::UartWriteError)?;
            let emic_status = if EMIC_STORAGE_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
            writeln!(uart, "  Status: {}", emic_status).map_err(|_| CliError::UartWriteError)?;
             if let Some(emic) = &mut EMIC_STORAGE_GLOBAL { // unsafe erişim
                 let blocks = emic.block_count().unwrap_or(0);
                 let bytes = blocks * BLOCK_SIZE as u64;
                 writeln!(uart, "  Total Blocks: {}", blocks).map_err(|_| CliError::UartWriteError)?;
                 writeln!(uart, "  Capacity: {} Bytes (approx {} MB)", bytes, bytes / 1024 / 1024).map_err(|_| CliError::UartWriteError)?;
             }
        },
        "sd" => {
             writeln!(uart, "SD Card Info:").map_err(|_| CliError::UartWriteError)?;
             let sd_status = if SD_CARD_STORAGE_GLOBAL.is_some() { "Initialized" } else { "Not Initialized" };
             writeln!(uart, "  Status: {}", sd_status).map_err(|_| CliError::UartWriteError)?;
             if let Some(sd) = &mut SD_CARD_STORAGE_GLOBAL { // unsafe erişim
                 let blocks = sd.block_count().unwrap_or(0);
                 let bytes = blocks * BLOCK_SIZE as u64;
                 writeln!(uart, "  Total Blocks: {}", blocks).map_err(|_| CliError::UartWriteError)?;
                 // 2.2TB limitini belirtmek faydali olabilir
                 writeln!(uart, "  Capacity: {} Bytes (approx {} MB / {} GB / {} TB - capped by MBR at ~2.2TB)", bytes, bytes / 1024 / 1024, bytes / 1024 / 1024 / 1024, bytes / 1024 / 1024 / 1024 / 1024).map_err(|_| CliError::UartWriteError)?;
             }
        },
        _ => {
            writeln!(uart, "Error: Unknown device type '{}'. Use 'emic' or 'sd'.", device_type).map_err(|_| CliError::UartWriteError)?;
        }
    }

    Ok(())
}


// boot komutu (Placeholder)
unsafe fn boot_command(uart: &mut Uart0, args: &[&str]) -> Result<(), CliError> {
    writeln!(uart, "Attempting to boot... (Placeholder)").map_err(|_| CliError::UartWriteError)?;

    // Burada boot sirasini belirleme (SD karttan mi, eMMC'den mi?)
    // Partition tablosunu okuma (MBR)
    // Bootable partition'i bulma
    // Bootloader'i bellege yukleme
    // Bootloader'a atlama

    if args.len() > 0 {
        writeln!(uart, "Ignoring arguments: {:?}", args).map_err(|_| CliError::UartWriteError)?;
    }

    // Örnek: SD karttan boot etmeye çalışma mantığı
    
    unsafe {
        if let Some(sd_card) = &mut SD_CARD_STORAGE_GLOBAL {
            log!("Checking SD card for bootloader...");
            // SD kartin ilk sektorunu oku (MBR)
            let mut mbr_buffer = [0u8; BLOCK_SIZE];
            match sd_card.read_block(0, &mut mbr_buffer) {
                Ok(_) => {
                    // MBR'yi parse etme logic'i (zor kisim!)
                    log!("MBR read OK. Parsing MBR...");
                    // Bootable partisyonu bul ve baslangic LBA'sini al
                    // Varsayalim ki bootloader 100. LBA'da
                    let bootloader_lba = 100;
                    let bootloader_size_blocks = 16; // Varsayim: 16 blok bootloader
                    let bootloader_dest_address = 0x9000_0000; // Varsayim: RAM'de yuklenecek yer

                    log!("Loading bootloader from LBA {}...", bootloader_lba);
                    // Bootloader bloklarini RAM'e oku
                    let mut bootloader_buffer = [0u8; BLOCK_SIZE * bootloader_size_blocks];
                    // Bu kismın BlockDevice trait'ine toplu okuma eklenmesi veya
                    // her blogun ayri ayri okunmasi gerekir.
                     for i in 0..bootloader_size_blocks {
                         match sd_card.read_block(bootloader_lba + i as u64, &mut bootloader_buffer[i*BLOCK_SIZE.. (i+1)*BLOCK_SIZE]) {
                             Ok(_) => {},
                             Err(e) => { log!("Failed to read bootloader block {}: {:?}", i, e); return Err(CliError::CommandFailed); }
                         }
                     }

                    // Bellege kopyala (varsayim)
                     core::ptr::copy_nonoverlapping(bootloader_buffer.as_ptr(), bootloader_dest_address as *mut u8, bootloader_buffer.len());
                    log!("Bootloader loaded to {:#x}.", bootloader_dest_address);

                    // Bootloader'a atla (unsafe!)
                     let bootloader_entry: extern "C" fn() = core::mem::transmute(bootloader_dest_address);
                     bootloader_entry(); // Buradan sonra kontrol bootloader'a gecer

                },
                Err(e) => {
                    log!("Failed to read MBR from SD card: {:?}", e);
                     writeln!(uart, "Failed to read MBR from SD card: {:?}", e).map_err(|_| CliError::UartWriteError)?;
                    return Err(CliError::CommandFailed);
                }
            }
        } else {
            log!("SD card not available for boot.");
             writeln!(uart, "SD card not available for boot.").map_err(|_| CliError::UartWriteError)?;
        }
    }
    

    // Boot basarili olduysa buraya donulmez.
    // Hata durumunda veya boot cihazi bulunamazsa buraya donulur.
    writeln!(uart, "Boot process finished or failed.").map_err(|_| CliError::UartWriteError)?;
    Ok(())
}

// --- Global CLI Instance ---
// Firmware'in ana döngüsünden erişim için.
// # Safety: Global mutable static kullanimi unsafe'dir.
pub static mut CLI_GLOBAL: Cli = Cli::new();