use std::io;
// use std::fmt;
use std::ops::Range;
use std::convert::TryFrom;

use num_derive::FromPrimitive;    
use num_traits::FromPrimitive;

/// Ostrich packet size, 1024 Bytes (1K)
pub const PCK_SIZE: usize = 1024;

/*  Ostrich packet format:
 *  Some fields are empty for some messages.
 *  For example, when sending an Error command
 *  fields sender and receiver are empty.
 *
 * 1   B : Command code (0)
 * 16  B : Sender name or empty (1-15) NOTE: First byte for text length
 * 16  B : Receiver or empty (16-31) NOTE: First byte for text length
 * 2   B : Text length in bytes (32-33)
 * 991 B : Text or empty (34-1023)
 */
pub const CMD_BYTES: Range<usize> = (0..0);
pub const SENDER_LEN: usize = 1;
pub const SENDER_BYTES: Range<usize> = (2..15);
pub const RECV_LEN: usize = 16;
pub const RECV_BYTES: Range<usize> = (17..31);
pub const TXT_LEN: Range<usize> = (32..33);
pub const TXT_BYTES: Range<usize> = (34..1023);

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum CommandCode {
    Ok = 0,
    Err = 1,
    Get = 2,
    Msg = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Ok,
    Err(String),
    Get,
    Msg(String, String, String),
}

pub struct RawMessage;

impl RawMessage {

    pub fn from_raw(raw: &[u8]) -> Result<Command, io::Error> {
        // Check the first byte for command code
        match FromPrimitive::from_u8(raw[0]) {
            Some(CommandCode::Ok) => Ok(Command::Ok),
            Some(CommandCode::Err) => {
                // Get error message length
                let mut range = [0u8;2];
                range[0] = raw[TXT_LEN.start];
                range[1] = raw[TXT_LEN.end];
                let n: usize = u16::from_ne_bytes(range) as usize;

                // Code the error into an utf-8 string
                let error = String::from_utf8_lossy(&raw[TXT_BYTES][..n]);
                Ok(Command::Err(error.to_string()))
            },
            Some(CommandCode::Get) => Ok(Command::Get),
            Some(CommandCode::Msg) => {
                let n = raw[SENDER_LEN] as usize;
                let sender = String::from_utf8_lossy(&raw[SENDER_BYTES][..n]);
                let n = raw[RECV_LEN] as usize;
                let recv = String::from_utf8_lossy(&raw[RECV_BYTES][..n]);
                
                // Get the length of the message text
                let mut range = [0u8;2];
                range[0] = raw[TXT_LEN.start];
                range[1] = raw[TXT_LEN.end];
                let n: usize = u16::from_ne_bytes(range) as usize;

                // Convert txt to string
                let text = String::from_utf8_lossy(&raw[TXT_BYTES][..n]);

                Ok(Command::Msg(sender.to_string(), recv.to_string(), text.to_string()))
            },
            None => Err(io::Error::new(io::ErrorKind::InvalidData, 
                                       format!("Incorrect command byte: {}", raw[0]))),
        }
    }

    fn put(buffer: &mut [u8], 
           content: &[u8], 
           range: Range<usize>) -> Result<(), io::Error> {
        
        // Check the range is inside the buffer's bounds
        if range.end > buffer.len() {
            let err = format!("Rage out of bounds: range end {}, buffer len {}", 
                              range.end, buffer.len());
            return Err(io::Error::new(io::ErrorKind::InvalidInput, err));
        }
        
        content.iter()
            .enumerate()
            .skip_while(|(i, _)| *i > range.end) // Check if the content size is larger than the range
            .for_each(|(i, x)| buffer[range.start+i] = *x);

        Ok(())
    }

    pub fn to_raw(command: &Command) -> Result<[u8; 1024], io::Error> {
        // Init buffer
        // let mut buffer = BytesMut::with_capacity(PCK_SIZE);
        let mut buffer = [0u8; PCK_SIZE];

        // Set command code
        match command {
            Command::Ok => buffer[0] = CommandCode::Ok as u8,
            Command::Err(err) => {
                // Set command code
                buffer[0] = CommandCode::Err as u8;

                // Set the error message length bytes
                let n = match u16::try_from(err.len()) {
                    Ok(n) => n.to_ne_bytes(),
                    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, 
                                                  "Error message length exceded"))
                };

                RawMessage::put(&mut buffer, &n, TXT_LEN)?;
                // Append the error's bytes to the buffer's text section
                let err = err.as_bytes();
                RawMessage::put(&mut buffer, err, TXT_BYTES)?;
            },
            Command::Get => buffer[0] = CommandCode::Get as u8,
            Command::Msg(s,r,t) => {
                // Append MSG code
                buffer[0] = CommandCode::Msg as u8;
                // Add sender name
                let s = s.as_bytes();
                buffer[SENDER_LEN] = s.len() as u8;
                RawMessage::put(&mut buffer, s, SENDER_BYTES)?;
                // Add receiver name
                let r = r.as_bytes();
                buffer[RECV_LEN] = r.len() as u8;
                RawMessage::put(&mut buffer, r, RECV_BYTES)?;

                // Set the txt message length bytes
                let t = t.as_bytes();
                let n = match u16::try_from(t.len()) {
                    Ok(n) => n.to_ne_bytes(),
                    Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, 
                                                  "Error message length exceded"))
                };
                RawMessage::put(&mut buffer, &n, TXT_LEN)?;
                // Add the messgase's body
                RawMessage::put(&mut buffer, t, TXT_BYTES)?;
            },
        }

        Ok(buffer)
    }
}

#[test]
fn test_ok() {
    // Test ok command
    let command = Command::Ok; 
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 0);
    assert_eq!(command, recovered);

}

#[test]
fn test_get() {
    // Test ok command
    let command = Command::Get; 
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 2);
    assert_eq!(command, recovered);

}

#[test]
fn test_err() {
    // Test error command
    let command = Command::Err("Some fatal error".to_string());
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 1);
    assert_eq!(command, recovered);
}

#[test]
fn test_msg() {
    // Test error command
    let command = Command::Msg("sender".to_string(),
                               "receiver".to_string(),
                               "The super secret message".to_string());

    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 3);
    assert_eq!(command, recovered);
}
