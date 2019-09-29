use serial_packet_parser::USARTPacket;
//use nucleo_f103rb::hal::stm32;

// constants for address range sizes
const CONFIG_ARRAY_SIZE: u8 = 64;
const DATA_ARRAY_SIZE: u8 = 60;
const COMMAND_COUNT: u8 = 12;

// constants for address starts
const DATA_ARRAY_START: u8 = 80;
const COMMAND_ADDR_START: u8 = 160;

// size of register array
const REGISTER_ARRAY_SIZE: usize = 160;

// start of FLASH config
const CONFIG_FLASH_ADDRESS: u32 = 0x0800_F000;
// start of FLASH factory
const FACTORY_FLASH_ADDRESS: u32 = 0x0800_E000;

pub enum USARTPacketType {
    Config,
    Data,
    Command,
    Unknown,
}

pub fn usartpacket_type(pkt: &USARTPacket) -> USARTPacketType {
    if pkt.address < CONFIG_ARRAY_SIZE {
        USARTPacketType::Config
    } else if pkt.address >= DATA_ARRAY_START && pkt.address < DATA_ARRAY_START + DATA_ARRAY_SIZE {
        USARTPacketType::Data
    } else if pkt.address >= COMMAND_ADDR_START && pkt.address < COMMAND_ADDR_START + COMMAND_COUNT
    {
        USARTPacketType::Command
    } else {
        USARTPacketType::Unknown
    }
}

pub struct Registers {
    registers: [u32; REGISTER_ARRAY_SIZE],
    peripheral: nucleo_f103rb::hal::stm32::Peripherals,
}

enum UseFlashStartAddress {
    Factory,
    Config,
}

impl Registers {
    pub fn new() -> Registers {
        /*
        // check the constants
        assert! (CONFIG_ARRAY_SIZE < DATA_ARRAY_START);
        assert! (COMMAND_ADDR_START > DATA_ARRAY_START + DATA_ARRAY_SIZE);
        assert! (REGISTER_ARRAY_SIZE == DATA_ARRAY_START + DATA_ARRAY_SIZE);
         */
        Registers {
            registers: [0; REGISTER_ARRAY_SIZE],
            peripheral: nucleo_f103rb::hal::stm32::Peripherals::take().unwrap(),
        }
    }

// #define FLASH_KEY1               ((uint32_t)0x45670123)
// #define FLASH_KEY2               ((uint32_t)0xCDEF89AB)

    fn flash_unlock(& mut self) {
        let flash = &self.peripheral.FLASH;
        /* Authorize the FPEC of Bank1 Access */
        flash.keyr.write(|w| unsafe { w.key().bits(0x45670123) });
        flash.keyr.write(|w| unsafe { w.key().bits(0xCDEF89AB) });
    }

    fn flash_lock(& mut self) {
        let flash = &self.peripheral.FLASH;
        /* Set the Lock Bit to lock the FPEC and the CR of  Bank1 */
        flash.cr.modify(|_r, w| w.lock().set_bit());
    }
    
    pub fn get_configuration(& mut self) {
        // read a byte from flash
        let flash_word: u32 = CONFIG_FLASH_ADDRESS;
        let pflash_word = &flash_word as *const u32;
        unsafe {
            if *pflash_word == 0xFFFF_FFFF {
                // flash is uninitialized
                self.reset_to_factory();
            } else {
                // flash is initialized
                self.load_configuration_from_flash(UseFlashStartAddress::Config);
            }
        }
    }

    unsafe fn load_configuration_from_flash(& mut self, which: UseFlashStartAddress) {
        let mut flash_addr: u32 = match which {
            UseFlashStartAddress::Factory => FACTORY_FLASH_ADDRESS,
            UseFlashStartAddress::Config => CONFIG_FLASH_ADDRESS,
        };
        for i in 0..CONFIG_ARRAY_SIZE {
            let j: u32 = (4 * i).into();
            flash_addr += j;
            let pflash_word = &flash_addr as *const u32;
            let index: usize = i.into();
            self.registers[index] = *pflash_word;
        }
    }

    fn reset_to_factory(& mut self) {
        // read a byte from flash
        let flash_word: u32 = FACTORY_FLASH_ADDRESS;
        let pflash_word = &flash_word as *const u32;
        unsafe {
            if *pflash_word == 0xFFFF_FFFF {
                // initialize to default values, since the factory area hasn't
                // been set
                self.registers = [0; REGISTER_ARRAY_SIZE];
            } else {
                // factory flash has default values, use those
                self.load_configuration_from_flash(UseFlashStartAddress::Factory);
            }
        }
    }

    pub fn clear_global_data(& mut self) {
        for i in 0..DATA_ARRAY_SIZE {
            let j: usize = (i + DATA_ARRAY_START).into();
            self.registers[j] = 0;
        }
    }

    pub fn write_configuration_to_flash(& mut self, which: UseFlashStartAddress) {
        let mut flash_addr: u32 = match which {
            UseFlashStartAddress::Factory => FACTORY_FLASH_ADDRESS,
            UseFlashStartAddress::Config => CONFIG_FLASH_ADDRESS,
        };
        while self.peripheral.FLASH.sr.read().bsy().bit() {
        }
        
        self.flash_unlock();

        // clear all pending flags
        // erase flash page in preparation for write
        // write config data

        self.flash_lock();
    }

    pub fn get(&self, address: usize) -> u32 {
        self.registers[address]
    }

    pub fn getf32(&self, address: usize) -> f32 {
        f32::from_bits(self.registers[address])
    }

    pub fn set(&mut self, address: usize, val: u32) {
        self.registers[address] = val;
    }

    pub fn setf32(&mut self, address: usize, val: f32) {
        self.registers[address] = val.to_bits();
    }
}
