use std::io;
use std::fmt;
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
 * 1   B : Sender's name length
 * 16  B : Sender name or empty
 * 1   B : Receiver's name length
 * 16  B : Receiver or empty
 * 2   B : Text length in bytes
 * 991 B : Text or empty
 */
pub const CMD_BYTES: Range<usize> = (0..0);
pub const SENDER_LEN: usize = 1;
pub const SENDER_BYTES: Range<usize> = (2..17);
pub const RECV_LEN: usize = 18;
pub const RECV_BYTES: Range<usize> = (19..34);
pub const TXT_LEN: Range<usize> = (35..36);
pub const TXT_BYTES: Range<usize> = (37..1023);

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u8)]
pub enum CommandCode {
    Ok = 0,
    Err = 1,
    Get = 2,
    Msg = 3,
    End = 4,
    Usr = 5, // User log in command
    Join = 6,
    Leave = 7,
    ListUsr = 8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListUsrOperation {
    Add,
    Remove,
}

// TODO : Descriptions
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // Ok
    Ok,
    // Error message: text (error)
    Err(String),                 
    // Get, NOTE: Maybe deleted in the future
    Get,
    // Send message: (sender, receiver, text)
    Msg(String, String, String),
    // End message
    End,
    // Log in user: sender (username), text (password)
    Usr(String, String),         
    // Join chat: group or user name
    Join(String),                
    // Leave chat: group or user name
    Leave(String),                
    // List groups: Group name, operation (add/remove users), strig of usernames of users
    ListUsr(String, ListUsrOperation, String)
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Ok => write!(f, "OK"),
            Command::Err(err) => write!(f, "ERROR: {}", err),
            Command::Get => write!(f, "GET"),
            Command::Msg(s, t, m) => write!(f, "{} -> {} : {}", s, t, m),
            Command::End => write!(f, "END"),
            Command::Usr(u, p) => write!(f, "USR: {}, PASWD: {}", u, p),
            Command::Join(gname) => write!(f, "JOIN: {}", gname),
            Command::Leave(gname) => write!(f, "LEAVE: {}", gname),
            Command::ListUsr(gname, ListUsrOperation::Add, users) => write!(f, 
                "LIST: group {} ADD users: {}", gname, users),
            Command::ListUsr(gname, ListUsrOperation::Remove, users) => write!(f, 
                "LIST: group {} REMOVE users: {}", gname, users),
        }
    }
}

pub struct RawMessage;

impl RawMessage {
    
    /// Parses the text segment of a given byte buffer into a string.
    /// It cares about the text length parameter given in the TXT_LEN 
    /// segment of the message.
    fn parse_text(raw: &[u8]) -> String {
        // Get the length of the message text
        let mut range = [0u8;2];
        range[0] = raw[TXT_LEN.start];
        range[1] = raw[TXT_LEN.end];
        let n: usize = u16::from_ne_bytes(range) as usize;
        // Convert txt to string
        let text = String::from_utf8_lossy(&raw[TXT_BYTES][..n]);
        text.to_string()
    }

pub fn from_raw(raw: &[u8]) -> Result<Command, io::Error> {
        // Check the first byte for command code
        match FromPrimitive::from_u8(raw[0]) {
            Some(CommandCode::Ok) => Ok(Command::Ok),
            Some(CommandCode::Err) => {
                // Get the error message from the text segment
                Ok(Command::Err(RawMessage::parse_text(&raw)))
            },
            Some(CommandCode::Get) => Ok(Command::Get),
            Some(CommandCode::Msg) => {
                // Get sender name
                let n = raw[SENDER_LEN] as usize;
                let sender = String::from_utf8_lossy(&raw[SENDER_BYTES][..n]);
                // Get receiver name
                let n = raw[RECV_LEN] as usize;
                let recv = String::from_utf8_lossy(&raw[RECV_BYTES][..n]);
                // Parse the message text into a string 
                let text = RawMessage::parse_text(&raw);

                Ok(Command::Msg(sender.to_string(), recv.to_string(), text))
            },
            Some(CommandCode::End) => Ok(Command::End),
            Some(CommandCode::Usr) => {
                // Get sender's username
                let n = raw[SENDER_LEN] as usize;
                let username = String::from_utf8_lossy(&raw[SENDER_BYTES][..n]);
                // Get password from the text segment 
                let password = RawMessage::parse_text(&raw);

                Ok(Command::Usr(username.to_string(), password))
            },
            Some(CommandCode::Join) => {
                // Get group's name length
                let n = raw[RECV_LEN] as usize;
                // Transform bytes to utf-8 string
                let gname = String::from_utf8_lossy(&raw[RECV_BYTES][..n]);
                
                Ok(Command::Join(gname.to_string()))
            },
            Some(CommandCode::Leave) => {
                // Get group's name length
                let n = raw[RECV_LEN] as usize;
                // Transform bytes to utf-8 string
                let gname = String::from_utf8_lossy(&raw[RECV_BYTES][..n]);
                
                Ok(Command::Leave(gname.to_string()))
            },
            Some(CommandCode::ListUsr) => {
                // Get group's name
                let n = raw[SENDER_LEN] as usize;
                let gname = String::from_utf8_lossy(&raw[SENDER_BYTES][..n]);
                // Operation is stored in RECV_LEN section
                let op = if raw[RECV_LEN] == 254 {
                    ListUsrOperation::Add
                } else if raw[RECV_LEN] == 0 {
                    ListUsrOperation::Remove
                } else {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                            "ListUsr command's operation specification section corrupted"));
                };
                // Parse the message text into a string 
                let text = RawMessage::parse_text(&raw);

                Ok(Command::ListUsr(gname.to_string(), op, text))
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
        
        if content.len() > range.len()+1 {
            let err = format!("Content larger than range, data loss might occur: range {:?}, content len {}", 
                              range, content.len());
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
                let err = err.as_bytes();
                let n = RawMessage::compute_text_length(&err)?;
                RawMessage::put(&mut buffer, &n, TXT_LEN)?;
                // Append the error's bytes to the buffer's text section
                RawMessage::put(&mut buffer, err, TXT_BYTES)?;
            },
            Command::Get => buffer[0] = CommandCode::Get as u8,
            Command::End => buffer[0] = CommandCode::End as u8,
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
                let n = RawMessage::compute_text_length(&t)?;
                RawMessage::put(&mut buffer, &n, TXT_LEN)?;
                // Add the messgase's body
                RawMessage::put(&mut buffer, t, TXT_BYTES)?;
            },
            Command::Usr(username, password) => {
                // Set USR command code
                buffer[0] = CommandCode::Usr as u8;
                // Set sender's username
                let username = username.as_bytes();
                buffer[SENDER_LEN] = username.len() as u8; // Set sender name size
                RawMessage::put(&mut buffer, username, SENDER_BYTES)?;
                // Set the password's buffer length
                let password = password.as_bytes(); 
                let length = RawMessage::compute_text_length(&password)?;
                RawMessage::put(&mut buffer, &length, TXT_LEN)?;
                // Set password in the text segment of the message
                RawMessage::put(&mut buffer, password, TXT_BYTES)?;
            },
            Command::Join(gname) => {
                // Set JOIN command code
                buffer[0] = CommandCode::Join as u8;
                // The name of the group to join is stored in the 
                // targets space of the message
                let gname = gname.as_bytes();
                buffer[RECV_LEN] = gname.len() as u8; // Set sender name size
                RawMessage::put(&mut buffer, gname, RECV_BYTES)?;
            },
            Command::Leave(gname) => {
                // Set LEAVE command code
                buffer[0] = CommandCode::Leave as u8;
                // The name of the group to leave is stored in the 
                // targets space of the message
                let gname = gname.as_bytes();
                buffer[RECV_LEN] = gname.len() as u8; // Set sender name size
                RawMessage::put(&mut buffer, gname, RECV_BYTES)?;
            },
            Command::ListUsr(gname, op, users) => {
                // Set ListUsr command code
                buffer[0] = CommandCode::ListUsr as u8;
                // Write gname's length and gname
                buffer[SENDER_LEN] = gname.as_bytes().len() as u8;
                RawMessage::put(&mut buffer, gname.as_bytes(), SENDER_BYTES)?;
                // Write the operation type in RECV_LEN section
                buffer[RECV_LEN] = match op {
                    ListUsrOperation::Add => 254,
                    ListUsrOperation::Remove => 0,
                };
                // Set the txt message length bytes
                let users = users.as_bytes();
                let n = RawMessage::compute_text_length(&users)?;
                RawMessage::put(&mut buffer, &n, TXT_LEN)?;
                // Add the messgase's body
                RawMessage::put(&mut buffer, users, TXT_BYTES)?;
            },
        }
        Ok(buffer)
    }
    
    /// Return's a 2Byte representation of the length of a given byte buffer.
    /// # Errors:
    /// Returns an InvalidInput error if the length of the buffer can 
    /// not be represented in two bytes.
    fn compute_text_length(buffer: &[u8]) -> Result<[u8;2], io::Error> {
        match u16::try_from(buffer.len()) {
            Ok(n) => Ok(n.to_ne_bytes()),
            Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, 
                                        "Error message length exceded"))
        }
    }
}

#[test]
fn test_ok() {
    let command = Command::Ok; 
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 0);
    assert_eq!(command, recovered);
}

#[test]
fn test_get() {
    let command = Command::Get; 
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 2);
    assert_eq!(command, recovered);
}

#[test]
fn test_err() {
    let command = Command::Err("Some fatal error".to_string());
    let mesg = RawMessage::to_raw(&command).unwrap();
    println!("command to raw ok");
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    println!("command from raw ok");
    assert_eq!(mesg[0], 1);
    assert_eq!(command, recovered);
}

#[test]
fn test_msg() {
    let command = Command::Msg("sender".to_string(),
                               "receiver".to_string(),
                               "The super secret message".to_string());

    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 3);
    assert_eq!(command, recovered);
}

#[test]
fn test_end() {
    let command = Command::End; 
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 4);
    assert_eq!(command, recovered);
}

#[test]
fn test_usr() {
    let command = Command::Usr("sender".to_string(),
                               "The super secret password".to_string());

    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 5);
    assert_eq!(command, recovered);
}

#[test]
fn test_join() {
    let command = Command::Join("#group_name".to_string());
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 6);
    assert_eq!(command, recovered);
}

#[test]
fn test_leave() {
    let command = Command::Leave("#group_name".to_string());
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 7);
    assert_eq!(command, recovered);
}

#[test]
fn test_listusr() {
    let command = Command::ListUsr("#group_name".to_string(), 
        ListUsrOperation::Add,
        "some\nmike\nkaixo\n".to_string());
    let mesg = RawMessage::to_raw(&command).unwrap();
    let recovered = RawMessage::from_raw(&mesg).unwrap();
    assert_eq!(mesg[0], 8);
    assert_eq!(command, recovered);
}
