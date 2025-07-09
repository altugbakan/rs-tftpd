use std::sync::Mutex;
use std::error::Error;

use crate::Packet;

static TX_DROP: Mutex<Vec<i32>> = Mutex::new(Vec::new());

pub fn drop_set(opt : Option<String>) -> Result<(), Box<dyn Error>> {
    if let Some(arg) = opt {
        let mut tx_drop = TX_DROP.lock().unwrap();
        for val in arg.split(',') {
            let val_num = val.parse::<i32>()?;
            tx_drop.push(val_num);
        }
        Ok(())
    } else {
        Err("Missing argument".into())
    }
}

fn check_seq_num(num: u16) -> bool
{
    let mut tx_drop = TX_DROP.lock().unwrap();
    if !tx_drop.is_empty() && tx_drop[0] == num as i32 {
        tx_drop.remove(0);
         return true;
    }
    false
}

pub fn drop_check(packet: &Packet) -> bool
{
    match packet {
        Packet::Data{block_num, data: _ } => check_seq_num(*block_num),
        Packet::Ack(block_num) => check_seq_num(*block_num),
        _ => false,
    }
}