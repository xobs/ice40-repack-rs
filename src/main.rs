// extern crate byteorder;
// use byteorder::ReadBytesExt;
//, WriteBytesExt, BigEndian, LittleEndian};

use std::fs::File;
use std::io::Read;
use std::cmp;

#[derive(Debug)]
enum FrequencyRange {
    Undefined,
    Low,
    Medium,
    High,
    Unknown(u32),
}

#[derive(PartialEq)]
enum FeatureFlag {
    Enabled,
    Disabled,
}

struct Ice40Bitstream {
    offset: u32,
    current_bank: u32,
    current_width: u32,
    current_height: u32,
    current_offset: u32,

    cram_width: u32,
    cram_height: u32,

    bram_width: u32,
    bram_height: u32,

    crc_value: u16,

    warmboot: FeatureFlag,
    nosleep: FeatureFlag,

    frequency_range: FrequencyRange,
}

impl Ice40Bitstream {
    pub fn from_file(mut input: File) -> Result<Self, ()> {

        let mut bs = Ice40Bitstream {
            offset: 0,
            current_bank: 0,
            current_width: 0,
            current_height: 0,
            current_offset: 0,

            cram_width: 0,
            cram_height: 0,

            bram_width: 0,
            bram_height: 0,

            crc_value: 0,

            warmboot: FeatureFlag::Disabled,
            nosleep: FeatureFlag::Disabled,

            frequency_range: FrequencyRange::Undefined,
        };
        let mut buffer = [0; 1];
        let mut preamble = 0;

        let mut wakeup = FeatureFlag::Disabled;

        // Look for the preamble
        loop {
            bs.offset = bs.offset + 1;

            input.read(&mut buffer[..]).expect("Couldn't read");
            preamble = preamble << 8;
            preamble = preamble | buffer[0] as u32;
            if preamble == 0x7EAA997E {
                println!("Found preamble at offset {}", bs.offset);
                break;
            }
        }

        // Parse commands
        while wakeup == FeatureFlag::Disabled {
            input.read(&mut buffer[..]).expect("Couldn't read");
            let cmd = buffer[0] >> 4;
            let payload_len = (buffer[0] & 0xf) as usize;
            let mut payload = 0;
            for _ in 0..payload_len {
                input.read(&mut buffer[..]).expect("Couldn't read");
                payload = payload << 8;
                payload = payload | buffer[0] as u32;
            }

            match cmd {
                0 => {
                    match payload {
                        0x01 => {
                            println!(
                                "CRAM Data [{}]: {} x {} bits = {} bits = {} bytes",
                                bs.current_bank,
                                bs.current_width,
                                bs.current_height,
                                bs.current_height * bs.current_width,
                                (bs.current_height * bs.current_width) / 8
                            );

                            bs.cram_width = cmp::max(bs.cram_width, bs.current_width);
                            bs.cram_height = cmp::max(bs.cram_height, bs.current_offset + bs.current_height);
                            /*
                            this->cram.resize(4);
                            this->cram[current_bank].resize(this->cram_width);
                            for (int x = 0; x < current_width; x++)
                                this->cram[current_bank][x].resize(this->cram_height);
                            */
                            for i in 0..(bs.current_height * bs.current_width) / 8 {
                                input.read(&mut buffer[..]).expect("Couldn't read");
                            }
                            /*
                            for (int j = 0; j < 8; j++) {
                                int x = (i*8 + j) % current_width;
                                int y = (i*8 + j) / current_width + current_offset;
                                this->cram[current_bank][x][y] = ((byte << j) & 0x80) != 0;
                            }
                            */
                            input.read(&mut buffer[..]).expect("Couldn't read");
                            let last0 = buffer[0];
                            input.read(&mut buffer[..]).expect("Couldn't read");
                            let last1 = buffer[0];

                            if (last0 != 0) || (last1 != 0) {
                                println!(
                                    "Expeded 0x0000 after CRAM data, got [{:x}] - [{:x}]\n",
                                    last0, last1
                                );
                            }
                        }

                        0x03 => {
                            println!(
                                "BRAM Data [{}]: {} x {} bits = {} bits = {} bytes",
                                bs.current_bank,
                                bs.current_width,
                                bs.current_height,
                                bs.current_height * bs.current_width,
                                (bs.current_height * bs.current_width) / 8
                            );
                            bs.bram_width = cmp::max(bs.bram_width, bs.current_width);
                            bs.bram_height = cmp::max(bs.bram_height, bs.current_offset + bs.current_height);
                            /*
                            this->bram.resize(4);
                            this->bram[current_bank].resize(this->bram_width);
                            for (int x = 0; x < current_width; x++)
                                this->bram[current_bank][x].resize(this->bram_height);
                            */
                            for i in 0..(bs.current_height * bs.current_width) / 8 {
                                input.read(&mut buffer[..]).expect("Couldn't read");
                            }
                            /*
                            for (int i = 0; i < (current_height*current_width)/8; i++) {
                                uint8_t byte = read_byte(ifs, crc_value, file_offset);
                                for (int j = 0; j < 8; j++) {
                                    int x = (i*8 + j) % current_width;
                                    int y = (i*8 + j) / current_width + current_offset;
                                    this->bram[current_bank][x][y] = ((byte << j) & 0x80) != 0;
                                }
                            }
                            */
                            input.read(&mut buffer[..]).expect("Couldn't read");
                            let last0 = buffer[0];
                            input.read(&mut buffer[..]).expect("Couldn't read");
                            let last1 = buffer[0];

                            if (last0 != 0) || (last1 != 0) {
                                println!(
                                    "Expeded 0x0000 after BRAM data, got [{:x}] - [{:x}]\n",
                                    last0, last1
                                );
                            }
                        }

                        0x05 => {
                            println!("Resetting CRC.\n");
                            bs.crc_value = 0xffff;
                        }

                        0x06 => {
                            println!("Wakeup.\n");
                            wakeup = FeatureFlag::Enabled;
                        }

                        x => {
                            panic!("Unknown command: 0x{:x} 0x{:x}\n", x, payload);
                        }
                    }
                }
                1 => {
                    bs.current_bank = payload;
                    println!("Setting current bank to {}", bs.current_bank);
                }
                2 => {
                    println!("CRC check");
                }
                5 => {
                    bs.frequency_range = match payload {
                        0 => FrequencyRange::Low,
                        1 => FrequencyRange::Medium,
                        2 => FrequencyRange::High,
                        x => FrequencyRange::Unknown(x),
                    };
                    println!("Setting frequency range to {:?}", bs.frequency_range);
                }
                3 => {
                    unimplemented!();
                }
                4 => {
                    unimplemented!();
                }
                6 => {
                    bs.current_width = payload + 1;
                    println!("Setting bank width to {}", bs.current_width);
                }
                7 => {
                    bs.current_height = payload;
                    println!("Setting bank height to {}", bs.current_height);
                }
                8 => {
                    bs.current_offset = payload;
                    println!("Setting bank offset to {}", bs.current_offset);
                }
                9 => match payload {
                    0 => {
                        bs.warmboot = FeatureFlag::Disabled;
                        bs.nosleep = FeatureFlag::Disabled;
                    }
                    1 => {
                        bs.warmboot = FeatureFlag::Disabled;
                        bs.nosleep = FeatureFlag::Enabled;
                    }
                    32 => {
                        bs.warmboot = FeatureFlag::Enabled;
                        bs.nosleep = FeatureFlag::Disabled;
                    }
                    33 => {
                        bs.warmboot = FeatureFlag::Enabled;
                        bs.nosleep = FeatureFlag::Enabled;
                    }
                    x => panic!("Unrecognized feature flags: {}", x),
                },
                y => {
                    panic!("Unrecognized command: {}", y);
                }
            }
        }
        Ok(bs)
    }
}
fn main() {
    println!("Hello, world!");
    let input = Ice40Bitstream::from_file(File::open("top.bin").expect("Couldn't find top.bin"))
        .expect("Couldn't parse file");
}
