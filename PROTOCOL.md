# Wire Protocol
The Ntrix VDC uses a custom protocol over a Serial BUS.

## Packet Types
There are three available packet types:

Control Packet:

The control packet is used to send commands to the VDC.

- It is sent as a packed two bytes in Big-Endian format.
- First 4 bits is a control code (CC)
- Remaining 12 bits stores a Control Code Payload (CC Payload)
- Some commands have no CC Payload

| Order | Name       | Length  |
| :---- | :--------- | :------ |
| 0     | CC         | 4 bits  |
| 1     | CC Payload | 12 bits |

Pixel Data Packet:

This contains pixel row data.

- The expected length depends on current display mode.
- Contains packed pixel data in current bits-per-pixel
- It is sent in Little-Endian format

Character Data Packet:

This contains Character Cell row data.

- The expected length depends on current display mode.
- Contains packed CharCells
- It is sent in Little-Endian format

## Display Modes
Available display modes. Sent in a GetMode or SetMode control packet.

- Even: Pixel Only
- Odd: Mixed (Pixel + Character)

| Mode | Dimensions | BPP |
| :--- |:---        | :-- |
| 0    | 640x480    | 1   |
| 1    | 80x60      | -   |
| 2    | 320x240    | 1   |
| 3    | 40x30      | -   |
| 4    | 160x120    | 1   |
| 5    | 20x15      | -   |
