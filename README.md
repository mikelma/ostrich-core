# ostrich core
This crate provides other ostrich projects commands of the ostrich protocol in order to communicate with each other. This commands are used to log in, send messages, join groups and much more. 

## Packet format
Commands are send in 1024 byte length TCP packets and all commands share the same packet layout. The 1024 bytes are dived in different sections:

| **Byte index**|  **Section**  |    **Description**   |
| ------------- |:-------------:| --------------------:|
| 0             | CMD_BYTES     | Command's ID         |
| 1             | SENDER_LEN    | Sender's name length |
| 2-17          | SENDER_BYES   | Sender's name        |
| 18            | RECV_LEN      | Receiver's length    |
| 19-34         | RECV_BYTES    | Receiver's name      |
| 35-36         | TXT_LEN       | Text section's length|
| 37-1023       | TXT_BYTES     | Text section         |
