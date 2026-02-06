# brio-smart-tech

## Acknowledgment

This repository is not affiliated with BRIO.

This project started as a fork of
[andiikaa/brio-bluetooth](https://github.com/andiikaa/brio-bluetooth) and has
now evolved to a crate that provides a class to interact with Brio SmartTech
child toy locomotive, over Bluetooth Low Energy.

The initial focus of the project was to experiment reverse engineering a
Bluetooth Low Energy protocol. The communication with the locomotive being
non-secured (this is not a critical application), it is a good choice for such
purpose.

Some [BRIO](https://www.brio.de/de-DE/produkte/eisenbahn/smart-tech-sound)
products allow the connection via Bluetooth and the control of their toys via
iOS or Android application.

A nice overview about the protocol specification can be found in
[cpetrich/Smart-Tech-Sound-BLE](https://github.com/cpetrich/Smart-Tech-Sound-BLE)

## Example

You can have access to some example code, which was used for testing in the
`example` folder.
