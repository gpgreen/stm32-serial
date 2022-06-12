use serial_packet_parser::USARTPacket;
use stm32f1xx_hal::pac;

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
    peripheral: pac::Peripherals,
}

pub enum UseFlashStartAddress {
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
            peripheral: pac::Peripherals::take().unwrap(),
        }
    }

    fn flash_unlock(&mut self) {
        /* Authorize the FPEC of Bank1 Access */
        self.peripheral
            .FLASH
            .keyr
            .write(|w| unsafe { w.key().bits(0x45670123) });
        self.peripheral
            .FLASH
            .keyr
            .write(|w| unsafe { w.key().bits(0xCDEF89AB) });
    }

    fn flash_lock(&mut self) {
        /* Set the Lock Bit to lock the FPEC and the CR of  Bank1 */
        self.peripheral.FLASH.cr.modify(|_r, w| w.lock().set_bit());
    }

    fn wait_till_not_busy(&mut self) -> u8 {
        let status: u8 = 0;
        while self.peripheral.FLASH.sr.read().bsy().bit() {}
        status
    }

    pub fn get_configuration(&mut self) {
        // read a byte from flash
        let flash_word: u32 = CONFIG_FLASH_ADDRESS;
        let pflash_word = &flash_word as *const u32;

        if unsafe { *pflash_word } == 0xFFFF_FFFF {
            // flash is uninitialized
            self.reset_to_factory();
        } else {
            // flash is initialized
            self.load_configuration_from_flash(UseFlashStartAddress::Config);
        }
    }

    fn load_configuration_from_flash(&mut self, which: UseFlashStartAddress) {
        let mut flash_addr: u32 = match which {
            UseFlashStartAddress::Factory => FACTORY_FLASH_ADDRESS,
            UseFlashStartAddress::Config => CONFIG_FLASH_ADDRESS,
        };
        for i in 0..CONFIG_ARRAY_SIZE {
            let j: u32 = (4 * i).into();
            flash_addr += j;
            let pflash_word = &flash_addr as *const u32;
            let index: usize = i.into();
            self.registers[index] = unsafe { *pflash_word };
        }
    }

    fn reset_to_factory(&mut self) {
        // read a byte from flash
        let flash_word: u32 = FACTORY_FLASH_ADDRESS;
        let pflash_word = &flash_word as *const u32;
        if unsafe { *pflash_word } == 0xFFFF_FFFF {
            // initialize to default values, since the factory area hasn't
            // been set
            self.registers = [0; REGISTER_ARRAY_SIZE];
        } else {
            // factory flash has default values, use those
            self.load_configuration_from_flash(UseFlashStartAddress::Factory);
        }
    }

    pub fn clear_global_data(&mut self) {
        for i in 0..DATA_ARRAY_SIZE {
            let j: usize = (i + DATA_ARRAY_START).into();
            self.registers[j] = 0;
        }
    }

    pub fn write_configuration_to_flash(&mut self, which: UseFlashStartAddress) {
        let flash_addr: u32 = match which {
            UseFlashStartAddress::Factory => FACTORY_FLASH_ADDRESS,
            UseFlashStartAddress::Config => CONFIG_FLASH_ADDRESS,
        };

        self.wait_till_not_busy();
        self.flash_unlock();

        // erase the page's
        for i in 0..CONFIG_ARRAY_SIZE {
            // erase the page
            let offset: u32 = (4 * i).into();
            let pg_flash_addr = flash_addr + offset;
            // enable the PER bit
            self.peripheral.FLASH.cr.modify(|_r, w| w.per().set_bit());
            self.peripheral
                .FLASH
                .ar
                .write(|w| unsafe { w.bits(pg_flash_addr) });
            // enable the START bit
            self.peripheral.FLASH.cr.modify(|_r, w| w.strt().set_bit());
            self.wait_till_not_busy();
            // disable the PER bit
            self.peripheral.FLASH.cr.modify(|_r, w| w.per().clear_bit());
        }
        // write config data
        for i in 0..CONFIG_ARRAY_SIZE {
            let j: usize = (4 * i).into();
            let offset: u32 = (4 * i).into();
            // write flash word - enable PG bit
            self.peripheral.FLASH.cr.modify(|_r, w| w.pg().set_bit());

            let mut wd_flash_addr = flash_addr + offset;
            let hi_bits: u16 = (self.registers[j] >> 16) as u16;
            let lo_bits: u16 = self.registers[j] as u16;

            // write first 16 bits
            let pflash_halfword = unsafe { &mut *(&mut wd_flash_addr as *mut u32 as *mut u16) };
            *pflash_halfword = hi_bits;
            self.wait_till_not_busy();

            // write last 16 bits
            wd_flash_addr = flash_addr + offset + 2;
            let pflash_halfword = unsafe { &mut *(&mut wd_flash_addr as *mut u32 as *mut u16) };
            *pflash_halfword = lo_bits;
            self.wait_till_not_busy();

            // disable the PG bit
            self.peripheral.FLASH.cr.modify(|_r, w| w.pg().clear_bit());

            // check that flash matches register
            wd_flash_addr = flash_addr + offset;
            let ptr = &wd_flash_addr as *const u32;
            if self.get(j) != unsafe { *ptr } {
                panic!("Written flash value doesn't match memory value");
            }
        }
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
