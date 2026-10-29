# Ola S1 vehicle-parser binder protocol: interoperability specification

Status: version 1.0, for the SDK host (GPL-3.0-or-later). Written as a clean-room
specification: it describes the wire contract only (names, order, types and integers).
It contains no source text, comments or implementation logic from any vendor code.

## 1. Scope

This document covers the Android binder protocol that the SDK host uses to talk to the
vehicle-parser service. It covers:

- The 2 methods of IVehicleService.
- The 15 methods of IParserService (all `subscribe*` calls). The host calls all 15.
- All 24 methods of ICommanderService. Only `sendTouchEvent` is called by the host. The
  others are specified so that the transaction codes can be reproduced exactly (section 8).
- All 191 methods of the 15 callback interfaces the host subscribes to. Every method is
  received, so every method is specified.
- The 37 enumerations and 3 parcelables that appear in those signatures.

Counts: 18 interfaces (3 service interfaces and 15 callback interfaces), 232 methods,
37 enumerations with 635 constants in total, 3 parcelables.

Not covered: the passcode/profile manager service used by the unlock path. It is a
separate binder service with its own descriptor and is outside this document (section 9).

## 2. Wire conventions

### 2.1 Parcel encoding

All values use the standard platform Parcel encoding. Integers are 32-bit, longs 64-bit,
floats and doubles IEEE 754, and strings are the platform string encoding (length prefix,
with a null value for an absent string). Booleans are written as an int: 1 for true, 0 for
false.

### 2.2 Interface token and descriptor

Every request and every callback starts with the interface token: the standard
writeInterfaceToken call with the descriptor string of the interface in question. The
receiving side verifies it with enforceInterface, and a mismatch rejects the call. Descriptors
are given in section 4 and must be reproduced exactly.

The descriptor query transaction (the platform's INTERFACE_TRANSACTION, code 1598968902) is
answered with the descriptor string. It is not part of the protocol surface.

### 2.3 Two-way and oneway calls

- Two-way calls: the reply begins with an int. 0 means no exception and the return value (if
  any) follows. A non-zero value means an exception parcel follows, as defined by the platform's
  readException.
- Oneway calls: the transaction flag FLAG_ONEWAY (value 1) is set. No reply is sent.

The mode of each method is given in section 5. Summary: all IVehicleService, IParserService and
ICommanderService methods are two-way. Of the 15 callback interfaces, 14 are oneway and
IVehicleServiceInfoCallbacks is two-way. A receiver of the two-way callbacks must send a reply
of int 0 after dispatching, or the sender waits.

### 2.4 Nullable objects

An enumeration or parcelable argument that may be null is written as a presence int: 0 for
null (nothing else follows), or 1 followed by the object's payload. The reader checks the int:
any value other than 0 means an object follows.

A callback argument (a binder object) is written as a strong binder, or null when absent.

### 2.5 Enumerations on the wire

An enumeration is written as an int equal to its zero-based index in the declared constant
order (section 6). The reader maps the int back by index. Constants are never written by name.
Adding, removing or reordering constants changes the wire, so the order in section 6 is part of
the contract.

### 2.6 Transaction codes

Transaction codes are absolute: the value in the "Code" column of each table is the value on the
wire. In platform terms, Code = FIRST_CALL_TRANSACTION + n, where n is the zero-based position of
the method. In every interface the codes are exactly 1 to N with no gaps. The platform numbers
methods by their declaration position, so a replacement interface must declare its methods in
exactly the Code-column order, with the same count.

## 3. Services and binding

### 3.1 Vehicle parser service

- Package: com.olaelectric.vehicleparser
- Service class: com.olaelectric.vehicleparser.services.VehicleParserService
- Intent action: com.olaelectric.vehicleparser.VehicleParserService

The host binds with an explicit component (package and class), the action, and the auto-create
binding flag. The returned binder is an IVehicleService.

### 3.2 Obtaining the other binders

IVehicleService is the only binder obtained from the bind. Its two calls return the other two
service binders:

- getParserServiceBinder (code 1) returns the IParserService binder.
- getCommanderServiceBinder (code 2) returns the ICommanderService binder.

Each returned binder must be wrapped with the matching interface.

### 3.3 Subscriptions

Every subscribe call takes a string key before the callback binder. The parser keys
subscriptions by this string. The host uses its own application package name as the key for all
15 subscriptions. A subscriber that reconnects should subscribe again. The subscribe calls return a
boolean.

## 4. Interface index

Every interface lives in the Java package `com.olaelectric.vehicleinterface`. The Java package and simple name to use are the
ones in the table. The descriptor is the package and simple name joined with a dot, and must be
exactly the value in the table (section 8, item 1).

| Interface | Java name | Descriptor | Direction | Mode | Host use |
|---|---|---|---|---|---|
| IVehicleService | com.olaelectric.vehicleinterface.IVehicleService | com.olaelectric.vehicleinterface.IVehicleService | host calls | two-way | 2 of 2 methods |
| IParserService | com.olaelectric.vehicleinterface.IParserService | com.olaelectric.vehicleinterface.IParserService | host calls | two-way | 15 of 15 methods |
| ICommanderService | com.olaelectric.vehicleinterface.ICommanderService | com.olaelectric.vehicleinterface.ICommanderService | host calls | two-way | 1 of 24 methods (sendTouchEvent) |

Callback interfaces: the host implements these and passes the binder to IParserService; the
parser calls them.

| Interface | Java name | Descriptor | Mode | Methods |
|---|---|---|---|---|
| IAlertInfoCallbacks | com.olaelectric.vehicleinterface.IAlertInfoCallbacks | com.olaelectric.vehicleinterface.IAlertInfoCallbacks | oneway | 8 |
| IBatteryInfoCallbacks | com.olaelectric.vehicleinterface.IBatteryInfoCallbacks | com.olaelectric.vehicleinterface.IBatteryInfoCallbacks | oneway | 25 |
| IChargerInfoCallbacks | com.olaelectric.vehicleinterface.IChargerInfoCallbacks | com.olaelectric.vehicleinterface.IChargerInfoCallbacks | oneway | 6 |
| IDisplayInfoCallbacks | com.olaelectric.vehicleinterface.IDisplayInfoCallbacks | com.olaelectric.vehicleinterface.IDisplayInfoCallbacks | oneway | 17 |
| IHandleBarCallbacks | com.olaelectric.vehicleinterface.IHandleBarCallbacks | com.olaelectric.vehicleinterface.IHandleBarCallbacks | oneway | 8 |
| IHardwareVersionInfoCallbacks | com.olaelectric.vehicleinterface.IHardwareVersionInfoCallbacks | com.olaelectric.vehicleinterface.IHardwareVersionInfoCallbacks | oneway | 15 |
| IImuInfoCallbacks | com.olaelectric.vehicleinterface.IImuInfoCallbacks | com.olaelectric.vehicleinterface.IImuInfoCallbacks | oneway | 3 |
| IMapInfoCallbacks | com.olaelectric.vehicleinterface.IMapInfoCallbacks | com.olaelectric.vehicleinterface.IMapInfoCallbacks | oneway | 3 |
| IMotorInfoCallbacks | com.olaelectric.vehicleinterface.IMotorInfoCallbacks | com.olaelectric.vehicleinterface.IMotorInfoCallbacks | oneway | 13 |
| IProximityCallbacks | com.olaelectric.vehicleinterface.IProximityCallbacks | com.olaelectric.vehicleinterface.IProximityCallbacks | oneway | 12 |
| IVehicleCommandCallbacks | com.olaelectric.vehicleinterface.IVehicleCommandCallbacks | com.olaelectric.vehicleinterface.IVehicleCommandCallbacks | oneway | 7 |
| IVehicleInfoCallbacks | com.olaelectric.vehicleinterface.IVehicleInfoCallbacks | com.olaelectric.vehicleinterface.IVehicleInfoCallbacks | oneway | 53 |
| IVehicleModeCallbacks | com.olaelectric.vehicleinterface.IVehicleModeCallbacks | com.olaelectric.vehicleinterface.IVehicleModeCallbacks | oneway | 9 |
| IVehicleServiceInfoCallbacks | com.olaelectric.vehicleinterface.IVehicleServiceInfoCallbacks | com.olaelectric.vehicleinterface.IVehicleServiceInfoCallbacks | two-way | 10 |
| IWidgetInfoCallbacks | com.olaelectric.vehicleinterface.IWidgetInfoCallbacks | com.olaelectric.vehicleinterface.IWidgetInfoCallbacks | oneway | 2 |

## 5. Method tables

### 5.1 IVehicleService

Descriptor `com.olaelectric.vehicleinterface.IVehicleService`.

| Code | Method | Arguments, in wire order | Reply | Host use | Mode |
|---|---|---|---|---|---|
| 1 | getParserServiceBinder | none | int 0, then strong binder | used | two-way |
| 2 | getCommanderServiceBinder | none | int 0, then strong binder | used | two-way |

### 5.2 IParserService

Descriptor `com.olaelectric.vehicleinterface.IParserService`. Every method takes the same pair of arguments: a
string key, then the callback's strong binder. Every method returns boolean.

| Code | Method | Arguments, in wire order | Reply | Host use | Mode |
|---|---|---|---|---|---|
| 1 | subscribeDisplayInfo | 1. String: string<br>2. IDisplayInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 2 | subscribeMotorInfo | 1. String: string<br>2. IMotorInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 3 | subscribeVehicleModes | 1. String: string<br>2. IVehicleModeCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 4 | subscribeBatteryInfo | 1. String: string<br>2. IBatteryInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 5 | subscribeHandleBar | 1. String: string<br>2. IHandleBarCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 6 | subscribeAlertInfo | 1. String: string<br>2. IAlertInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 7 | subscribeIMUInfo | 1. String: string<br>2. IImuInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 8 | subscribeVehicleCommand | 1. String: string<br>2. IVehicleCommandCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 9 | subscribeProximityInfo | 1. String: string<br>2. IProximityCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 10 | subscribeHardwareVersionInfo | 1. String: string<br>2. IHardwareVersionInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 11 | subscribeWidgetData | 1. String: string<br>2. IWidgetInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 12 | subscribeVehicleInfo | 1. String: string<br>2. IVehicleInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 13 | subscribeMapInfo | 1. String: string<br>2. IMapInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 14 | subscribeChargerInfo | 1. String: string<br>2. IChargerInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |
| 15 | subscribeVehicleServiceInfo | 1. String: string<br>2. IVehicleServiceInfoCallbacks: strong binder of the callback object (asBinder(); null allowed) | int 0, then int (1 = true, 0 = false) | used | two-way |

### 5.3 ICommanderService

Descriptor `com.olaelectric.vehicleinterface.ICommanderService`. The host calls `sendTouchEvent` once per second while
the screen is on, and calls no other method. The stock unlock path does not use this interface
(section 8, item 6).

| Code | Method | Arguments, in wire order | Reply | Host use | Mode |
|---|---|---|---|---|---|
| 1 | unlockedByPasscode | 1. boolean: int (1 = true, 0 = false) | int 0, then int (1 = true, 0 = false) | not used | two-way |
| 2 | sendSomState | 1. SomState: nullable: int 0 for null, or int 1 followed by int ordinal of SomState (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 3 | navigationStatus | 1. boolean: int (1 = true, 0 = false) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 4 | sendCommandToScooter | 1. CommandFromHMI: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromHMI (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 5 | sendCloudCommandToScooter | 1. CommandFromApp: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromApp (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 6 | sendCtaHmiStatus | 1. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 7 | setScooterBrightnessLevel | 1. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 8 | sendVehicleSafetyAlertAck | 1. VehicleSafetyAlerts: nullable: int 0 for null, or int 1 followed by int ordinal of VehicleSafetyAlerts (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 9 | setFollowMeToHome | 1. boolean: int (1 = true, 0 = false)<br>2. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 10 | sendTouchEvent | 1. boolean: int (1 = true, 0 = false) | int 0 (no-exception marker) and nothing else | used | two-way |
| 11 | sendVehicleCommandAck | 1. CommandFromVehicle: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromVehicle (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 12 | getProximityAddress | none | int 0, then nullable BleDetails: int 0 for null, or int 1 followed by its fields | not used | two-way |
| 13 | getProximityVer | none | int 0, then string | not used | two-way |
| 14 | sendProximityCmd | 1. ProximityCommandType: nullable: int 0 for null, or int 1 followed by int ordinal of ProximityCommandType (section 6)<br>2. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 15 | sendPartyModeCmd | 1. boolean: int (1 = true, 0 = false)<br>2. boolean: int (1 = true, 0 = false)<br>3. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 16 | sendAcktoBLE | 1. MapCommandAck: nullable: int 0 for null, or int 1 followed by int ordinal of MapCommandAck (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 17 | sendMagnetometerValueToBCM | 1. long: long (8 bytes)<br>2. long: long (8 bytes)<br>3. long: long (8 bytes) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 18 | sendSOCLimitToScooter | 1. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 19 | sendAckPasscodeCmd | 1. int: int | int 0 (no-exception marker) and nothing else | not used | two-way |
| 20 | sendHardwareVariant | 1. int: int<br>2. int: int<br>3. boolean: int (1 = true, 0 = false) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 21 | sendHyperChargerDueStatus | 1. ChargerDueStatus: nullable: int 0 for null, or int 1 followed by int ordinal of ChargerDueStatus (section 6)<br>2. boolean: int (1 = true, 0 = false) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 22 | sendCustomModeInfo | 1. String: string | int 0 (no-exception marker) and nothing else | not used | two-way |
| 23 | sendKrutrimCommandToScooter | 1. CommandFromKrutrim: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromKrutrim (section 6) | int 0 (no-exception marker) and nothing else | not used | two-way |
| 24 | sendEpochTimeToScooter | 1. String: string | int 0 (no-exception marker) and nothing else | not used | two-way |

### 5.4 Callback interfaces

Callback objects receive the interface token first, then the arguments in the order shown. Each
method's code is its transaction number within that interface.

#### IAlertInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IAlertInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onSideStandPostionAlert | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 2 | onSteeringLockUnLockAlert | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 3 | onWakeUpReason | 1. HUMessage: nullable: int 0 for null, or int 1 followed by int ordinal of HUMessage (section 6) | none | oneway |
| 4 | onFallDetectionAlert | 1. FallDetection: nullable: int 0 for null, or int 1 followed by int ordinal of FallDetection (section 6) | none | oneway |
| 5 | onTowTheftDetectionAlert | 1. TowTheftDetection: nullable: int 0 for null, or int 1 followed by int ordinal of TowTheftDetection (section 6) | none | oneway |
| 6 | onRegenAvailabilityAlert | 1. RegenAvailability: nullable: int 0 for null, or int 1 followed by int ordinal of RegenAvailability (section 6) | none | oneway |
| 7 | onHillholdAvailablityAlert | 1. HillHoldStatus: nullable: int 0 for null, or int 1 followed by int ordinal of HillHoldStatus (section 6) | none | oneway |
| 8 | onTamperPseudoWakeupStateAlert | 1. boolean: int (1 = true, 0 = false) | none | oneway |

#### IBatteryInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IBatteryInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onTimeToChargeSC80Changed | 1. int: int | none | oneway |
| 2 | onTimeToChargeSC00Changed | 1. int: int | none | oneway |
| 3 | onTimeToFC100Changed | 1. int: int | none | oneway |
| 4 | onTimeToChargeFCOptimumLevelChanged | 1. int: int | none | oneway |
| 5 | onEffectiveBatteryTemperatureChanged | 1. float: float (4 bytes) | none | oneway |
| 6 | onMinBatteryTemperatureChanged | 1. float: float (4 bytes) | none | oneway |
| 7 | onMaxBatteryTemperatureChanged | 1. float: float (4 bytes) | none | oneway |
| 8 | onBatteryCurrentChanged | 1. float: float (4 bytes) | none | oneway |
| 9 | onBatteryPackTemperature1Changed | 1. float: float (4 bytes) | none | oneway |
| 10 | onBatteryPackTemperature2Changed | 1. float: float (4 bytes) | none | oneway |
| 11 | onBatterPackSocChanged | 1. float: float (4 bytes) | none | oneway |
| 12 | onAvailableCapacityChanged | 1. float: float (4 bytes) | none | oneway |
| 13 | isRegenAvailabilityFlag | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 14 | onBatteryTemperatureDisplay | 1. int: int | none | oneway |
| 15 | onOverallSocChanged | 1. float: float (4 bytes) | none | oneway |
| 16 | onTotalKwhrDisCharged | 1. float: float (4 bytes) | none | oneway |
| 17 | onTotalKwhrCharged | 1. float: float (4 bytes) | none | oneway |
| 18 | onTotalRegenerative | 1. float: float (4 bytes) | none | oneway |
| 19 | onBatteryCurrent | 1. float: float (4 bytes) | none | oneway |
| 20 | onBusVoltage | 1. float: float (4 bytes) | none | oneway |
| 21 | onPackSoH | 1. int: int | none | oneway |
| 22 | onBMSModeUpdated | 1. BMSModes: nullable: int 0 for null, or int 1 followed by int ordinal of BMSModes (section 6) | none | oneway |
| 23 | onBatteryCellTypeReceived | 1. BatteryCellType: nullable: int 0 for null, or int 1 followed by int ordinal of BatteryCellType (section 6) | none | oneway |
| 24 | onBMSResetCounterUpdated | 1. long: long (8 bytes) | none | oneway |
| 25 | onBatteryIdUpdated | 1. String: string | none | oneway |

#### IChargerInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IChargerInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | chargerBillingStatus | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 2 | onEvseIdReceived | 1. String: string | none | oneway |
| 3 | onChargerConnected | 1. ChargerTypes: nullable: int 0 for null, or int 1 followed by int ordinal of ChargerTypes (section 6) | none | oneway |
| 4 | onChargerPluggedIn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 5 | onChargerSerialNumberReceived | 1. String: string | none | oneway |
| 6 | onVehicleConnectorLatched | 1. boolean: int (1 = true, 0 = false) | none | oneway |

#### IDisplayInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IDisplayInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | isLeftIndicatorOn | 1. boolean: int (1 = true, 0 = false)<br>2. IndicatorFault: nullable: int 0 for null, or int 1 followed by int ordinal of IndicatorFault (section 6) | none | oneway |
| 2 | isRightIndicatorOn | 1. boolean: int (1 = true, 0 = false)<br>2. IndicatorFault: nullable: int 0 for null, or int 1 followed by int ordinal of IndicatorFault (section 6) | none | oneway |
| 3 | isHighBeamOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 4 | isPositionLightOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 5 | isSideStandOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 6 | isTrunkLock | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 7 | onRangeChanged | 1. int: int | none | oneway |
| 8 | onBatterySocChanged | 1. int: int | none | oneway |
| 9 | onChargingStatusChanged | 1. ChargingStatus: nullable: int 0 for null, or int 1 followed by int ordinal of ChargingStatus (section 6) | none | oneway |
| 10 | onBrakeWornOutAlert | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 11 | onBrightnessLevelChanged | 1. int: int | none | oneway |
| 12 | isLowBeamOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 13 | onVehicleRangeBasedOnModesChanged | 1. VehicleRangeBasedOnModes: nullable: int 0 for null, or int 1 followed by the fields of VehicleRangeBasedOnModes (section 7) | none | oneway |
| 14 | onOdoValueChangedInMeters | 1. long: long (8 bytes) | none | oneway |
| 15 | onHeadUnitSupplyTurnedOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 16 | onMcuSupplyTurnedOn | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 17 | onDiyModeRangeChanged | 1. int: int | none | oneway |

#### IHandleBarCallbacks

Descriptor `com.olaelectric.vehicleinterface.IHandleBarCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onDpad_StartOrStopStartOrStop | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 2 | onDpad_Drive_Mode | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 3 | onDpad_AcceptOrConfirmOrVolumeUp | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 4 | onDpad_RejectOrCancelOrVolumeDown | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 5 | onDpad_PlayOrPauseOrVoiceAssist | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 6 | onAcceptConfirmLongPress | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 7 | onRejectCancelLongPress | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 8 | onPlayPauseLongPress | 1. boolean: int (1 = true, 0 = false) | none | oneway |

#### IHardwareVersionInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IHardwareVersionInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onBCMversionChanged | 1. String: string | none | oneway |
| 2 | onBMSversionChanged | 1. String: string | none | oneway |
| 3 | onHUversionChanged | 1. String: string | none | oneway |
| 4 | onHBCversionChanged | 1. String: string | none | oneway |
| 5 | onMCUversionChanged | 1. String: string | none | oneway |
| 6 | onBCMBootloaderVersionChanged | 1. String: string | none | oneway |
| 7 | onBMSBootloaderVersionChanged | 1. String: string | none | oneway |
| 8 | onHUBootloaderVersionChanged | 1. String: string | none | oneway |
| 9 | onESCLVersionChanged | 1. String: string | none | oneway |
| 10 | onBCMHardwareVariantChanged | 1. String: string | none | oneway |
| 11 | onBMSHardwareVariantChanged | 1. String: string | none | oneway |
| 12 | onBMSDipSwitchVariantChanged | 1. String: string | none | oneway |
| 13 | onMCUHardwareVariantChanged | 1. String: string | none | oneway |
| 14 | onChargerHardwareVariantChanged | 1. String: string | none | oneway |
| 15 | onMotorHardwareVariantChanged | 1. String: string | none | oneway |

#### IImuInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IImuInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onIMUXDataChanged | 1. IMUData: nullable: int 0 for null, or int 1 followed by the fields of IMUData (section 7) | none | oneway |
| 2 | onIMUYDataChanged | 1. IMUData: nullable: int 0 for null, or int 1 followed by the fields of IMUData (section 7) | none | oneway |
| 3 | onIMUZDataChanged | 1. IMUData: nullable: int 0 for null, or int 1 followed by the fields of IMUData (section 7) | none | oneway |

#### IMapInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IMapInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onLatitudeUpdated | 1. double: double (8 bytes) | none | oneway |
| 2 | onLongtitudeUpdated | 1. double: double (8 bytes) | none | oneway |
| 3 | onOlaMapsPOIUpdated | 1. int: int<br>2. String: string | none | oneway |

#### IMotorInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IMotorInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onSpeedChanged | 1. int: int | none | oneway |
| 2 | onMotorRpmChanged | 1. int: int | none | oneway |
| 3 | onMotorTorqueChanged | 1. float: float (4 bytes) | none | oneway |
| 4 | isCruiseControlEngaged | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 5 | onDistanceCovered | 1. int: int | none | oneway |
| 6 | onRegenNotificatioReceived | 1. RegenStatus: nullable: int 0 for null, or int 1 followed by int ordinal of RegenStatus (section 6) | none | oneway |
| 7 | onVehicleDischargeModeChanged | 1. DriveModes: nullable: int 0 for null, or int 1 followed by int ordinal of DriveModes (section 6) | none | oneway |
| 8 | onVehicleSpeedChanged | 1. float: float (4 bytes) | none | oneway |
| 9 | onThrottleValue1Changed | 1. float: float (4 bytes) | none | oneway |
| 10 | onMotorTemperatureChanged | 1. float: float (4 bytes) | none | oneway |
| 11 | onVehicleDirectionChanged | 1. int: int | none | oneway |
| 12 | onMcuTemperatureChanged | 1. float: float (4 bytes) | none | oneway |
| 13 | onMotorCurrentChanged | 1. int: int | none | oneway |

#### IProximityCallbacks

Descriptor `com.olaelectric.vehicleinterface.IProximityCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onProximityCommandTypeChanged | 1. ProximityCommandType: nullable: int 0 for null, or int 1 followed by int ordinal of ProximityCommandType (section 6) | none | oneway |
| 2 | onPassCodeDataChanged | 1. int: int | none | oneway |
| 3 | onProximityAckChanged | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 4 | onProximityVersionUpdated | 1. String: string | none | oneway |
| 5 | onPartyModeCommandReceived | 1. int: int<br>2. int: int | none | oneway |
| 6 | onProfileIDReceived | 1. int: int | none | oneway |
| 7 | onPromityFeatureEnabledState | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 8 | onDeletedProfileIdAckReceived | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 9 | onLockUnlockReasonChanged | 1. LockUnlockReason: nullable: int 0 for null, or int 1 followed by int ordinal of LockUnlockReason (section 6) | none | oneway |
| 10 | onPasscodeUpdated | 1. int: int<br>2. String: string | none | oneway |
| 11 | onAppNotificationInfoReceived | 1. String: string | none | oneway |
| 12 | onBleStatusMessageReceived | 1. BleCappConnectionStatus: nullable: int 0 for null, or int 1 followed by int ordinal of BleCappConnectionStatus (section 6) | none | oneway |

#### IVehicleCommandCallbacks

Descriptor `com.olaelectric.vehicleinterface.IVehicleCommandCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onCommandFromVehicle | 1. CommandFromVehicle: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromVehicle (section 6) | none | oneway |
| 2 | onACKCommandFromVehicle | 1. CommandFromVehicle: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromVehicle (section 6) | none | oneway |
| 3 | sendCommandAck | 1. CommandFromHMI: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromHMI (section 6) | none | oneway |
| 4 | sendCloudCommandAck | 1. CommandFromApp: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromApp (section 6) | none | oneway |
| 5 | onPasscodeWindowActivated | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 6 | onVariantAckReceived | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 7 | onAckCommandFromKrutrim | 1. CommandFromKrutrim: nullable: int 0 for null, or int 1 followed by int ordinal of CommandFromKrutrim (section 6) | none | oneway |

#### IVehicleInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IVehicleInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onVinInfoReceived | 1. String: string | none | oneway |
| 2 | onVehicleTypeReceived | 1. VehicleTypes: nullable: int 0 for null, or int 1 followed by int ordinal of VehicleTypes (section 6) | none | oneway |
| 3 | onProximityDetected | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 4 | isCruiseReverseMode | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 5 | isPowerButtonPressed | 1. PowerButtonStatus: nullable: int 0 for null, or int 1 followed by int ordinal of PowerButtonStatus (section 6) | none | oneway |
| 6 | isFrontBrakeApplied | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 7 | isRearBrakeApplied | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 8 | onDriveModeChanged | 1. DriveModes: nullable: int 0 for null, or int 1 followed by int ordinal of DriveModes (section 6) | none | oneway |
| 9 | onBoardNavigation | 1. OnboardNavigation: nullable: int 0 for null, or int 1 followed by int ordinal of OnboardNavigation (section 6) | none | oneway |
| 10 | onTakeMeHomeLightsEnabled | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 11 | onVehicleAcceleration | 1. float: float (4 bytes) | none | oneway |
| 12 | isBoostModeFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 13 | isFallDetectionFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 14 | isTowTamperDetectionFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 15 | isProximityLockFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 16 | isHillHoldFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 17 | isSmartHeadLightFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 18 | isSmartDRLLightFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 19 | isAutoHeadLightFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 20 | isHazardLightFeatureFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 21 | isAutoIndicatorTurnOffFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 22 | isMotorCutoffDelayFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 23 | isFallDetectedL1Phase | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 24 | isFallDetectedL2Phase | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 25 | onAccidentSeverityStatus | 1. AccidentSeverityStatus: nullable: int 0 for null, or int 1 followed by int ordinal of AccidentSeverityStatus (section 6) | none | oneway |
| 26 | onVehicleInclinationChanged | 1. int: int | none | oneway |
| 27 | isBMSBoardVariantSWMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 28 | isCellChemistrySWMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 29 | isDipSwitchNotSentError | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 30 | isVinVariantMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 31 | isVariantDipSwitchMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 32 | isBCMBMSMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 33 | isSOMBMSMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 34 | isMCUSWVariantMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 35 | isBCMSWVersionVariantMismatch | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 36 | onHardBrakingCount | 1. long: long (8 bytes) | none | oneway |
| 37 | onHardBrakingFlag | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 38 | isVacationModeFeatureAvailable | 1. boolean: int (1 = true, 0 = false) | none | oneway |
| 39 | isAdvancedRegenTPFeatureAvailable | 1. int: int | none | oneway |
| 40 | isFastChargerTPFeatureAvailable | 1. int: int | none | oneway |
| 41 | isHyperChargerTPFeatureAvailable | 1. int: int | none | oneway |
| 42 | isEnergyInsightsTPFeatureAvailable | 1. int: int | none | oneway |
| 43 | isProximityLUTPFeatureAvailable | 1. int: int | none | oneway |
| 44 | isGeoFencingTPFeatureAvailable | 1. int: int | none | oneway |
| 45 | isTimeFencingTPFeatureAvailable | 1. int: int | none | oneway |
| 46 | isModeFencingTPFeatureAvailable | 1. int: int | none | oneway |
| 47 | isNewUpgradesTPFeatureAvailable | 1. int: int | none | oneway |
| 48 | isEarlyBetaAccessTPFeatureAvailable | 1. int: int | none | oneway |
| 49 | isHillHoldTPFeatureAvailable | 1. int: int | none | oneway |
| 50 | isTowProtectionTPFeatureAvailable | 1. int: int | none | oneway |
| 51 | isSmartParkFeatureAvailable | 1. int: int | none | oneway |
| 52 | onMBMSWakeUpReason | 1. MBMSWakeUpReason: nullable: int 0 for null, or int 1 followed by int ordinal of MBMSWakeUpReason (section 6) | none | oneway |
| 53 | onVCUResetCounterUpdated | 1. long: long (8 bytes) | none | oneway |

#### IVehicleModeCallbacks

Descriptor `com.olaelectric.vehicleinterface.IVehicleModeCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onOdoMeterChanged | 1. int: int | none | oneway |
| 2 | onVehicleStateChanged | 1. VehicleStates: nullable: int 0 for null, or int 1 followed by int ordinal of VehicleStates (section 6) | none | oneway |
| 3 | onVehicleChargingModeChanged | 1. ChargingModes: nullable: int 0 for null, or int 1 followed by int ordinal of ChargingModes (section 6) | none | oneway |
| 4 | onAvailableDrivingModesChanged | 1. AvailableDrivingModes: nullable: int 0 for null, or int 1 followed by int ordinal of AvailableDrivingModes (section 6) | none | oneway |
| 5 | onCruiseControlAvailabilityChanged | 1. CruiseControlAvailability: nullable: int 0 for null, or int 1 followed by int ordinal of CruiseControlAvailability (section 6) | none | oneway |
| 6 | onVehicleLockStateBCMChanged | 1. VehicleLockStateBCM: nullable: int 0 for null, or int 1 followed by int ordinal of VehicleLockStateBCM (section 6) | none | oneway |
| 7 | onVehicleModeLevel4Changed | 1. DriveModes: nullable: int 0 for null, or int 1 followed by int ordinal of DriveModes (section 6) | none | oneway |
| 8 | onHmiStatusBarSttChanged | 1. HmiStatusBarStt: nullable: int 0 for null, or int 1 followed by int ordinal of HmiStatusBarStt (section 6) | none | oneway |
| 9 | onECUModeChanged | 1. ECUModes: nullable: int 0 for null, or int 1 followed by int ordinal of ECUModes (section 6) | none | oneway |

#### IVehicleServiceInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IVehicleServiceInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onPartMismatchDistLeft | 1. int: int | int 0 (no-exception marker) and nothing else | two-way |
| 2 | onJobOpenDistLeft | 1. int: int | int 0 (no-exception marker) and nothing else | two-way |
| 3 | onDeliveryDistLeft | 1. int: int | int 0 (no-exception marker) and nothing else | two-way |
| 4 | onModeCappingOdoDistLeft | 1. int: int | int 0 (no-exception marker) and nothing else | two-way |
| 5 | onServiceRestrictedMode | 1. boolean: int (1 = true, 0 = false) | int 0 (no-exception marker) and nothing else | two-way |
| 6 | onVehicleMonitorMode | 1. VehicleMonitorMode: nullable: int 0 for null, or int 1 followed by int ordinal of VehicleMonitorMode (section 6) | int 0 (no-exception marker) and nothing else | two-way |
| 7 | onMaxOdoLimit | 1. int: int | int 0 (no-exception marker) and nothing else | two-way |
| 8 | onInPurchaseModeLimit | 1. ModeLimitStatus: nullable: int 0 for null, or int 1 followed by int ordinal of ModeLimitStatus (section 6) | int 0 (no-exception marker) and nothing else | two-way |
| 9 | onServiceModeLimit | 1. ModeLimitStatus: nullable: int 0 for null, or int 1 followed by int ordinal of ModeLimitStatus (section 6) | int 0 (no-exception marker) and nothing else | two-way |
| 10 | onPartsMismatchModeLimit | 1. ModeLimitStatus: nullable: int 0 for null, or int 1 followed by int ordinal of ModeLimitStatus (section 6) | int 0 (no-exception marker) and nothing else | two-way |

#### IWidgetInfoCallbacks

Descriptor `com.olaelectric.vehicleinterface.IWidgetInfoCallbacks`. Subscribed by the host through IParserService; every method is received and logged.

| Code | Method | Arguments, in wire order | Reply | Mode |
|---|---|---|---|---|
| 1 | onCo2AvoidedValueChanged | 1. int: int | none | oneway |
| 2 | onTreeGrowthValueChanged | 1. int: int | none | oneway |


## 6. Enumerations

Summary of all 37 enumerations:

| Enumeration | Constants | Read path | Host use |
|---|---|---|---|
| AccidentSeverityStatus | 7 | index lookup; out-of-range index fails | yes |
| AvailableDrivingModes | 5 | index lookup; out-of-range index fails | no |
| BMSModes | 9 | index lookup; out-of-range index fails | no |
| BatteryCellType | 8 | index lookup; out-of-range index fails | no |
| BleCappConnectionStatus | 4 | index lookup; out-of-range index fails | no |
| ChargerDueStatus | 4 | index lookup; out-of-range index fails | no |
| ChargerTypes | 4 | index lookup; out-of-range index fails | no |
| ChargingModes | 7 | index lookup; out-of-range index fails | no |
| ChargingStatus | 4 | index lookup; out-of-range index fails | yes |
| CommandFromApp | 22 | index lookup; out-of-range index fails | no |
| CommandFromHMI | 107 | index lookup; out-of-range index fails | no |
| CommandFromKrutrim | 14 | index lookup; out-of-range index fails | no |
| CommandFromVehicle | 217 | index lookup; out-of-range index fails | yes |
| CruiseControlAvailability | 5 | index lookup; out-of-range index fails | no |
| DriveModes | 12 | index lookup; out-of-range index fails | yes |
| ECUModes | 8 | index lookup; out-of-range index fails | no |
| FallDetection | 4 | index lookup; out-of-range index fails | yes |
| HUMessage | 2 | index lookup; out-of-range index fails | no |
| HillHoldStatus | 7 | index lookup; out-of-range index fails | no |
| HmiStatusBarStt | 6 | index lookup; out-of-range index fails | no |
| IndicatorFault | 12 | index lookup; out-of-range index fails | yes |
| LockUnlockReason | 8 | index lookup; out-of-range index fails | no |
| MBMSWakeUpReason | 8 | index lookup; out-of-range index fails | no |
| MapCommandAck | 3 | index lookup; out-of-range index fails | no |
| ModeLimitStatus | 3 | index lookup; out-of-range index fails | no |
| OnboardNavigation | 2 | index lookup; out-of-range index fails | no |
| PowerButtonStatus | 4 | index lookup; out-of-range index fails | no |
| ProximityCommandType | 65 | index lookup for 0-64; explicit mapping for 199-238 (6.1); other values fail | no |
| RegenAvailability | 4 | index lookup; out-of-range index fails | no |
| RegenStatus | 5 | index lookup; out-of-range index fails | yes |
| SomState | 7 | index lookup; out-of-range index fails | no |
| TowTheftDetection | 3 | index lookup; out-of-range index fails | yes |
| VehicleLockStateBCM | 5 | index lookup; out-of-range index fails | yes |
| VehicleMonitorMode | 4 | index lookup; out-of-range index fails | no |
| VehicleSafetyAlerts | 13 | index lookup; out-of-range index fails | no |
| VehicleStates | 16 | index lookup; out-of-range index fails | yes |
| VehicleTypes | 17 | index lookup; out-of-range index fails | no |

Every enumeration is written as its index (section 2.5). The read path is an index lookup with no
default: an index outside the constant list fails the read. Implementers that must tolerate unknown
values choose their own fallback, but must still accept every index in the lists.

### 6.1 ProximityCommandType: additional read mapping

The stock reader also accepts the integers below, mapping each to the named constant. Writers
still write the index. Values 0 to 64 use the index lookup; the values below map as shown; any
other value fails the read.

| Read int | Constant returned |
|---|---|
| 199 | PAIRING_POPUP_ACK |
| 200 | EXPLICITE_COMMAND_PAIRING |
| 201 | PROXIMITY_PAIRING_ACK |
| 202 | HILL_HOLD_RQST_Enable_Selected |
| 203 | HILL_HOLD_RQST_Disable_Selected |
| 204 | REGEN_TYPE_DEFAULT_Selected |
| 205 | REGEN_TYPE_CITY_RIDE_EN_Selected |
| 206 | REGEN_TYPE_HIGHWAY_GLIDE_EN_Selected |
| 207 | SMART_HEAD_LIGHT_DEFAULT_Selected |
| 208 | SMART_HEAD_LIGHT_LOW_BEAM_EN_Selected |
| 209 | SMART_HEAD_LIGHT_HIGH_BEAM_EN_Selected |
| 210 | SMART_HEAD_LIGHT_SMART_LIGHT_EN_Selected |
| 211 | BOOST_MODE_Enable_Selected |
| 212 | BOOST_MODE_Disable_Selected |
| 213 | FALL_DETECTION_Enable_Selected |
| 214 | FALL_DETECTION_Disable_Selected |
| 215 | TOW_TAMPER_DETECTION_Enable_Selected |
| 216 | TOW_TAMPER_DETECTION_Disable_Selected |
| 217 | PROXIMITY_LOCK_UNLOCK_Enable_Selected |
| 218 | PROXIMITY_LOCK_UNLOCK_Disable_Selected |
| 219 | SMART_DRL_LIGHT_Enable_Selected |
| 220 | SMART_DRL_LIGHT_Disable_Selected |
| 221 | AUTO_HEADLIGHT_HANDLING_Enable_Selected |
| 222 | AUTO_HEADLIGHT_HANDLING_Disable_Selected |
| 223 | HAZARD_LIGHT_FEATURE_Enable_Selected |
| 224 | HAZARD_LIGHT_FEATURE_Disable_Selected |
| 225 | AUTO_INDICATOR_TURN_OFF_FEATURE_Enable_Selected |
| 226 | AUTO_INDICATOR_TURN_OFF_FEATURE_Disable_Selected |
| 227 | MOTOR_CUTOFF_DELAY_Enable_Selected |
| 228 | MOTOR_CUTOFF_DELAY_Disable_Selected |
| 229 | Hill_Hold_Request_Enable_Selected |
| 230 | Hill_Hold_Request_Disable_Selected |
| 231 | Vacation_Mode_ON |
| 232 | Vacation_Mode_OFF |
| 233 | Need_Help |
| 234 | I_am_Safe |
| 235 | Delete_Profile |
| 236 | User_Calibration_Request |
| 237 | TMH_Lights_Enable |
| 238 | TMH_Lights_Disabled |

### 6.2 Constant tables

#### AccidentSeverityStatus

Java name `com.olaelectric.vehicleinterface.AccidentSeverityStatus`. Host use: yes. Wire: int ordinal (section 2.5). 7 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NO_ACCIDENT | 1 | MINOR_PARK | 2 | MINOR_AWAKE | 3 | MAJOR_ZERO_SPD |
| 4 | MAJOR_LOW_SPD | 5 | SEVERE_LOW_SPD | 6 | SEVERE_HIGH_SPD |  |  |

#### AvailableDrivingModes

Java name `com.olaelectric.vehicleinterface.AvailableDrivingModes`. Host use: no. Wire: int ordinal (section 2.5). 5 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | ALL_MODES_AVAILABLE | 1 | SPO_HYP_NOT_AVAILABLE | 2 | NOR_SPO_HYP_NOT_AVAILABLE | 3 | NO_MODES_AVAILABLE |
| 4 | RESERVE |  |  |  |  |  |  |

#### BMSModes

Java name `com.olaelectric.vehicleinterface.BMSModes`. Host use: no. Wire: int ordinal (section 2.5). 9 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | Initialise | 1 | Sleep_Pending | 2 | Reserved | 3 | Standby |
| 4 | Charge | 5 | Discharge | 6 | Sleep | 7 | Fault_Or_Error |
| 8 | Invalid |  |  |  |  |  |  |

#### BatteryCellType

Java name `com.olaelectric.vehicleinterface.BatteryCellType`. Host use: no. Wire: int ordinal (section 2.5). 8 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SAMSUNG | 1 | LG | 2 | BAK | 3 | StoreDot |
| 4 | PTLLFP | 5 | Reserved1 | 6 | Reserved2 | 7 | Reserved3 |

#### BleCappConnectionStatus

Java name `com.olaelectric.vehicleinterface.BleCappConnectionStatus`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | INVALID | 1 | BLE_CAPP_CONNECTED | 2 | BLE_CAPP_NOT_CONNECTED | 3 | BLE_CAPP_CONNECTION_IN_PROGRESS |

#### ChargerDueStatus

Java name `com.olaelectric.vehicleinterface.ChargerDueStatus`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | DUES_PENDING | 2 | DUES_PAID | 3 | CONNECTION_FAILED |

#### ChargerTypes

Java name `com.olaelectric.vehicleinterface.ChargerTypes`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NONE | 1 | FAST_CHARGER | 2 | SLOW_CHARGER | 3 | BOOST_CHARGER |

#### ChargingModes

Java name `com.olaelectric.vehicleinterface.ChargingModes`. Host use: no. Wire: int ordinal (section 2.5). 7 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | Idle | 2 | Safe | 3 | Slow_Charge |
| 4 | Fast_Charge | 5 | Discharge | 6 | Boost_Charge |  |  |

#### ChargingStatus

Java name `com.olaelectric.vehicleinterface.ChargingStatus`. Host use: yes. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | OFF | 1 | ON_ERROR | 2 | ON_COMPLETED | 3 | ON_IN_PROGRESS |

#### CommandFromApp

Java name `com.olaelectric.vehicleinterface.CommandFromApp`. Host use: no. Wire: int ordinal (section 2.5). 22 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | No_Command | 1 | Vehicle_UNLOCK | 2 | Vehicle_LOCK | 3 | Vehicle_HeadLamp_ON |
| 4 | Vehicle_HeadLamps_OFF | 5 | Trunk_Open | 6 | Vehicle_Remote_Immobilize_ON | 7 | Vehicle_Remote_Immobilize_OFF |
| 8 | Lock_without_Steering | 9 | Reserved | 10 | Diagnostic_Request | 11 | Find_My_Bike_Light_Turn_ON_Request |
| 12 | Find_My_Bike_Sound_Turn_ON_Request | 13 | PRNTCNTRL_SPORT_ACS_REVOKED | 14 | PRNTCNTRL_HYPER_ACS_REVOKED | 15 | PRNTCNTRL_ACS_RESET |
| 16 | Alarm_unlock_Now | 17 | Accident_Detection_Enable_Selected | 18 | Accident_Detection_Disable_Selected | 19 | Tow_Tamper_Detection_Enable_Selected |
| 20 | Tow_Tamper_Detection_Disable_Selected | 21 | FMB_SPEAKER_ENABLE |  |  |  |  |

#### CommandFromHMI

Java name `com.olaelectric.vehicleinterface.CommandFromHMI`. Host use: no. Wire: int ordinal (section 2.5). 107 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | No_Command | 1 | Lock_with_Steering | 2 | Lock_without_Steering | 3 | Seat_Unlock_Cmd |
| 4 | Reverse_mode_activated | 5 | Headlight_Turn_ON_Request | 6 | ECO_Mode_Rejected_by_User | 7 | ECO_Mode_Accepted_by_User |
| 8 | Vehicle_Shutdown_is_Rejected_by_User | 9 | ALARM_Turn_Off_Request | 10 | Headlight_Turn_OFF_Request | 11 | Accepted_Get_Home_Request |
| 12 | Rejected_Get_Home_Request | 13 | Stop_Charge | 14 | Scooter_Reboot | 15 | OTA_Installation_Started |
| 16 | OTA_Installation_in_Progress | 17 | OTA_Installation_Complete | 18 | MCU_Reboot | 19 | HU_Reboot |
| 20 | Passcode_ACK | 21 | TripoMeter_Reset_Successfull | 22 | CTA_LOCK | 23 | CTA_UNLOCK |
| 24 | CTA_IGNORE | 25 | PASSCODE_AUTH | 26 | High_Beam_ON | 27 | High_Beam_OFF |
| 28 | Low_Beam_ON | 29 | Low_Beam_OFF | 30 | Rear_Light_ON | 31 | Rear_Light_OFF |
| 32 | Left_Indicator_ON | 33 | Left_Indicator_OFF | 34 | Right_Indicator_ON | 35 | Right_Indicator_OFF |
| 36 | Front_DRL_ON | 37 | Front_DRL_OFF | 38 | AUTO_HEADLIGHT_SUNRISE_EN | 39 | Auto_HEAD_LIGHT_SUNSET_EN |
| 40 | Auto_Head_Light_Reserved | 41 | Regen_Type_Default_SELECTED | 42 | Regen_Type_City_Ride_En_Selected | 43 | Regen_Type_Highway_Glide_En_Selected |
| 44 | Smart_Head_Light_Default_Selected | 45 | Smart_Head_Light_Low_Beam_En_Selected | 46 | Smart_Head_Light_High_Beam_En_Selected | 47 | Smart_Head_Light_Smart_Light_En_Selected |
| 48 | Boost_Mode_Enable_Selected | 49 | Boost_Mode_Disable_Selected | 50 | Accident_Detection_Enable_Selected | 51 | Accident_Detection_Disable_Selected |
| 52 | Tow_Tamper_Detection_Enable_Selected | 53 | Tow_Tamper_Detection_Disable_Selected | 54 | Proximity_Lock_Unlock_Enable_Selected | 55 | Proximity_Lock_Unlock_Disable_Selected |
| 56 | Smart_DRL_Light_Enable_Selected | 57 | Smart_DRL_Light_Disable_Selected | 58 | Auto_Head_Light_Handling_Enable_Selected | 59 | Auto_Head_Light_Handling_Disable_Selected |
| 60 | Hazard_Light_Feature_Enable_Selected | 61 | Hazard_Light_Feature_Disable_Selected | 62 | Auto_Indicator_Turn_off_Feature_Enable_Selected | 63 | Auto_Indicator_Turn_off_Feature_Disable_Selected |
| 64 | Motor_Cutoff_Delay_Selected_Enable_Selected | 65 | Motor_Cutoff_Delay_Selected_Disable_Selected | 66 | Hill_Hold_Request_Enable_Selected | 67 | Hill_Hold_Request_Disable_Selected |
| 68 | VACATION_MODE_ON | 69 | VACATION_MODE_OFF | 70 | NEED_HELP | 71 | I_AM_SAFE |
| 72 | DELETE_PROFILE | 73 | USER_CALIBRATION_REQUEST | 74 | Hazard_Light_Turn_ON | 75 | Hazard_Light_Turn_OFF |
| 76 | HMI_Alarm_ignore_True | 77 | Tow_Theft_Low_Sensitivity | 78 | Tow_Theft_High_Sensitivity | 79 | HMI_Alarm_ignore_False |
| 80 | Exit_Diagnostic_Mode | 81 | Version_Mismatched | 82 | Smart_Head_Light_SMART_LIGHT_EN_Deselected | 83 | Ride_Now |
| 84 | Lock_Now | 85 | TMH_Lights_Enable | 86 | TMH_Lights_Disabled | 87 | Alarm_unlock_Now |
| 88 | Drive_acess_revoked_enable | 89 | Drive_acess_revoked_disable | 90 | SIDE_STAND_DISABLE | 91 | SIDE_STAND_ENABLE |
| 92 | Brake_Switch_Fault_Deactivate | 93 | Brake_Switch_Fault_Activate | 94 | AI_Ind_OFF | 95 | Brk_Snsr_Fail_detect_AI |
| 96 | Brk_Snsr_Fail_clear_AI | 97 | OTA_MCU_Reset | 98 | HMI_Wakeup_pass | 99 | Restrict_Drive |
| 100 | SOS_Enable | 101 | SOS_Disable | 102 | BD_Drv_Prk_Ack | 103 | Park_Assist_Enabled |
| 104 | Park_Assist_Disabled | 105 | DIY_mode_Enable | 106 | DIY_mode_Disable |  |  |

#### CommandFromKrutrim

Java name `com.olaelectric.vehicleinterface.CommandFromKrutrim`. Host use: no. Wire: int ordinal (section 2.5). 14 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | LockTheScooter | 2 | HighBeamON | 3 | HighBeamOFF |
| 4 | RightIndicatorON | 5 | RightIndicatorOFF | 6 | LeftIndicatorON | 7 | LeftIndicatorOFF |
| 8 | IncreaseVolume | 9 | DecreaseVolume | 10 | TURN_ON_NORMAL_MODE | 11 | TURN_ON_SPORT_MODE |
| 12 | TURN_ON_HYPER_MODE | 13 | TURN_ON_ECO_MODE |  |  |  |  |

#### CommandFromVehicle

Java name `com.olaelectric.vehicleinterface.CommandFromVehicle`. Host use: yes. Wire: int ordinal (section 2.5). 217 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NO_COMMAND | 1 | ACTIVATE_PASSCODE_WINDOW | 2 | DERATE_AFE_FAULT_CCS | 3 | DERATE_PDU_COOLDOWN_WAIT |
| 4 | DERATE_CHARGE_TO_RIDE | 5 | DISABLED_PDU_MODE_SWITCH | 6 | DISABLED_HIGHER_MODE | 7 | DERATE_AFE_FAULT |
| 8 | RERATING_REVIVAL_PDU | 9 | DERATE_PDU_TEMP_ERROR_START | 10 | DERATE_PDU_TEMP_ERROR_INPGRS | 11 | DERATE_SOC_3_WARNING |
| 12 | DERATE_SOC_3 | 13 | DERATE_PDU_TEMP_WARN_START | 14 | DERATE_PDU_TEMP_WARN_INPGRS | 15 | RERATING_HIGHER_MODE |
| 16 | DERATE_SOC_15_WARNING | 17 | TYPE3_ERROR_SET | 18 | TYPE3_ERROR_CLEAR | 19 | TYPE2_ERROR_SET |
| 20 | TYPE2_ERROR_CLEAR | 21 | TYPE1_ERROR_SET | 22 | TYPE1_ERROR_CLEAR | 23 | TRUNK_OPEN_NOTIFICATION |
| 24 | TRUNK_CLOSE_NOTIFCATION | 25 | ENTERING_INTO_DEEP_SLEEP | 26 | TEST_01 | 27 | TEST_02 |
| 28 | TEST_03 | 29 | TEST_04 | 30 | TEST_05 | 31 | DERATE_SOC_15 |
| 32 | DERATE_SOC_20_WARNING | 33 | DERATE_SOC_20 | 34 | GENERAL_LOW_BATTERY | 35 | DERATE_OVER_TEMP_WARN_START |
| 36 | DERATE_OVER_TEMP_WARN_INPGRS | 37 | DERATE_OVER_TEMP_ERR_START | 38 | DERATE_OVER_TEMP_ERR_INPGRS | 39 | DERATE_TEMP_COOLDOWN_WAIT |
| 40 | DISABLED_OT_MODE_SWITCH | 41 | RERATING_REVIVAL_OT | 42 | CHARGE_NOT_HAPPENING | 43 | DERATE_MOT_TEMP_ERROR_START |
| 44 | DERATE_MCU_TEMP_ERROR_START | 45 | DERATE_MOT_COOLDOWN_WAIT | 46 | DERATE_MCU_COOLDOWN_WAIT | 47 | DISABLED_MOT_MODE_SWITCH |
| 48 | DISABLED_MCU_MODE_SWITCH | 49 | DERATE_MOT_TEMP_WARN_START | 50 | DERATE_MCU_TEMP_WARN_START | 51 | RERATING_HIGHER_MODE_OT |
| 52 | RERATING_HIGHER_MODE_MOT | 53 | RERATING_HIGHER_MODE_MCU | 54 | DERATE_MOT_CHILL_WARN | 55 | DERATE_MCU_CHILL_WARN |
| 56 | RERATING_REVIVAL_MOT | 57 | RERATING_REVIVAL_MCU | 58 | DERATE_CHRG_OVER_TEMP_WARN | 59 | CLOSE_SIDE_STAND_TO_DRIVE |
| 60 | SIDE_STAND_OPEN | 61 | APPLY_SIDE_STAND_TO_LOCK | 62 | HOLD_BRK_SSB_TO_DRV | 63 | BATT_TEMP_OOR |
| 64 | BATT_TEMP_OOR_ALL | 65 | CURR_SNSR_FAIL_OC_SC | 66 | VLT_SNSR_FAIL | 67 | VLT_SNSR_FAIL_ALL |
| 68 | BATT_TEMP_SNSR_FAIL | 69 | BATT_TEMP_SNSR_FAIL_ALL | 70 | OVER_CURR_CHRG_WARN | 71 | MCU_GRADE2_ECO |
| 72 | MCU_GRADE2_RESTART | 73 | MCU_GRADE2_CONTACT_CS | 74 | MCU_GRADE3_PARK | 75 | MCU_GRADE3_RESTART |
| 76 | MCU_GRADE3_CONTACT_CS | 77 | BATT_SNSR_FAIL_CONTACT_CS | 78 | BATT_TEMP_OOR_CHRG | 79 | BATT_TEMP_OOR_ALL_CHRG |
| 80 | VLT_SNSR_FAIL_CHRG | 81 | VLT_SNSR_FAIL_ALL_CHRG | 82 | BATT_TEMP_SNSR_FAIL_CHRG | 83 | BATT_TEMP_SNSR_FAIL_ALL_CHRG |
| 84 | CHARGE_BATT_LOWTEMP_WARN | 85 | DERATE_LOW_TEMP_WARN_START | 86 | DERATE_LOW_TEMP_ERR_START | 87 | PDU_WARN_CHRG |
| 88 | MCU_GRADE2_DISABLED_HIGHER | 89 | INITIATE_TRIPOMETER_RESET | 90 | INITIATE_NAVIGATION_PING_TEST | 91 | CELL_DIP_ERR_GRADE2 |
| 92 | SPIKE_ERR_GRADE2 | 93 | CAN_FAIL | 94 | RERATE_HIGHER_MODE_DRV_TEMP | 95 | RERATE_HIGHER_MODE_CHRG_TEMP |
| 96 | RERATE_HIGHER_MODE_CELL_DIP | 97 | RERATE_HIGHER_MODE_SPIK_ERR | 98 | RERATING_REVIVAL_CAN | 99 | SLP_TEMP_RISE_GRADE2 |
| 100 | RERATE_HIGHER_MODE_SLP_OT | 101 | DRV_TEMP_RISE_GRADE2 | 102 | CHRG_TEMP_RISE_GRADE2 | 103 | DISABLED_CELLDIP_MODE_SWITCH |
| 104 | DISABLED_SPIKE_MODE_SWITCH | 105 | DISABLED_TEMP_MODE_RISE_SWITCH | 106 | CAN_FAIL_WAIT_CONTACT_CS | 107 | BMS_GRADE4_CUTOFF |
| 108 | CNTCR_TOGGLING_ISSUE | 109 | OVR_CHRG_PROTECTION | 110 | LOCK_IN_PRGS | 111 | CHRG_CMPLT_UNPLG_CHRGR |
| 112 | STOP_CHRG_OVER_TEMP_ERR | 113 | NORMAL_MODE_AVAILABLE | 114 | VEHICLE_FALL_DETECTED | 115 | VEHICLE_FALL_RECOVERED |
| 116 | VEHICLE_FALL_ALARM_IGNORED | 117 | USER_CALIBRATION_SUCCESS | 118 | USER_CALIBRATION_FAIL | 119 | UNLOCK_FAIL_CTA |
| 120 | LOCK_FAIL_CTA | 121 | CHGR_PERMANENT_FAULT | 122 | CHGR_TEMPORARY_FAULT | 123 | CHGR_PWR_OFF |
| 124 | CHGR_LATCH_NOT_CONED | 125 | RID_ACCS_RVKED_PARK_IN_30S | 126 | DERATE_THROT_ERROR | 127 | DERATE_DRV_FLT |
| 128 | DCHRG_INHBT_ERROR | 129 | HILL_HOLD_EXIT_NUDGE | 130 | UNPLUG_CHGR_TO_RIDE | 131 | SAFE_UNPLUG_HYPER_CHARGER |
| 132 | INITIALIZING_HYPERCHARGER | 133 | UNPLUG_HYPER_CHGR_FAULT | 134 | HILL_HOLD_USR_DEACT_REQ | 135 | ENTERING_VACATION_MODE |
| 136 | EXITING_VACATION_MODE | 137 | PLEASE_RELEASE_EMERGENCY_BUTTON_TO_START_CHARGING | 138 | ACCS_RVKED_CONTACT_OWNER | 139 | CONFIG_MISMATCH |
| 140 | VERSION_MISMATCH | 141 | THERMAL_RUNAWAY | 142 | DISABLED_SPORTS_MODE_FAULT | 143 | DISABLED_SPORTS_MODE_SOC |
| 144 | DISABLED_SPORTS_MODE_TEMP | 145 | GEOFEN_ACS_REVOKED | 146 | PRNTCNTRL_SPORT_ACS_REVOKED | 147 | PRNTCNTRL_HYPER_ACS_REVOKED |
| 148 | PRNTCNTRL_ACS_RESET | 149 | VCU_Over_Temp_Grade1 | 150 | MCU_Temp_Rise_Grade1 | 151 | MOTOR_Temp_Rise_Grade1 |
| 152 | Accident_Detected | 153 | TAMPER_PASS_EXIT | 154 | LOW_BATTERY_DEACTIVATING_CRUISE | 155 | LOW_BATTERY_CANNOT_ENABLE_CRUISE |
| 156 | CRUISE_DEACT_BATT_CURR_SPIKE | 157 | CHARGE_BATT_LOWTEMP_ERR | 158 | DCDC_LOW_THRESH_REACHED | 159 | CHARGING_PAYMENT_DUE |
| 160 | SideStand_DownWarning | 161 | CAN_FAILURE_CANT_ENABLE_CRUISE | 162 | CAN_COM_FAILURE | 163 | CAN_COM_FAILURE_GRADE3_PRK |
| 164 | CAN_COM_ERROR_NORMAL | 165 | REDUCED_PERFORMANCE_CANNOT_ENABLE_CRUISE | 166 | THROTTLE_OFFSET_DRV_ERROR | 167 | PDU_WARN_ECO |
| 168 | PDU_WARN_ECO_CRUISE_FAILURE | 169 | PDU_WARN_ECO_START | 170 | RELEASE_THROTTLE_TO_DRIVE | 171 | BATT_WARN_ECO |
| 172 | BATT_WARN_ECO_CRUISE_FAILURE | 173 | BATT_WARN_ECO_START | 174 | USE_ECO_NUDGE | 175 | USE_REGEN_NUDGE |
| 176 | UPGRADE_FOR_BOOSTCHARGER | 177 | UPGRADE_FOR_FASTCHARGER | 178 | UPGRADE_FOR_HILLHOLD | 179 | TECHPACK_AVAILABLE |
| 180 | NOT_DELIVERED_EXHAUSTED_RANGE | 181 | SCOOTER_NOT_DELIVERED | 182 | IN_SERVICE_RANGE_EXHAUST | 183 | IN_SERVICE |
| 184 | PARTS_MISMATCHED | 185 | PARTS_MISMATCHED_EXHAUSTED | 186 | ACCIDENT_EMERGENCY_WINDOW | 187 | ENABLE_RIDE_TO_REVERSE |
| 188 | SOC_DERATION_DRIVE_TO_PARK | 189 | SOC_DERATION_UNLOCK | 190 | BBW_BRAKE_FAILURE_DRIVE | 191 | BBW_BRAKE_FAILURE_PARK |
| 192 | BBW_BREAKSENSOR_FAULT | 193 | BLE_MAC_GRADE1 | 194 | PM_SCHEME_NOTIF_1 | 195 | PM_SCHEME_NOTIF_2 |
| 196 | PM_SCHEME_NOTIF_3 | 197 | PM_SCHEME_NOTIF_4 | 198 | PM_SCHEME_NOTIF_5 | 199 | TEST_RIDE |
| 200 | BRAKE_PRESSED_LMP_WARN | 201 | ENABLE_RIDE_TO_SWITCH_MODE | 202 | COME_TO_HALT_TO_TURNOFF | 203 | TP_HYPER_NA |
| 204 | TP_SPORT_NA | 205 | TP_ECO_NA | 206 | TP_REVERSE_NA | 207 | TP_CRUISE_NA |
| 208 | TP_CHARGING_NA | 209 | NOT_DELIVER_PARK_TRANSITION | 210 | JOB_OPEN_PARK_TRANSITION | 211 | JOB_OPEN_RANGE_EXHAUST |
| 212 | NOT_DELIVERED_RANGE_EXHAUST | 213 | NOT_DELIVER_RESTRICTED_ECO | 214 | NOT_DELIVER_RESTRICTED_NORMAL | 215 | JOB_OPEN_RESTRICTED_ECO |
| 216 | JOB_OPEN_RESTRICTED_NORMAL |  |  |  |  |  |  |

#### CruiseControlAvailability

Java name `com.olaelectric.vehicleinterface.CruiseControlAvailability`. Host use: no. Wire: int ordinal (section 2.5). 5 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | AVAILABLE_NOT_ACTV | 1 | ACTIV | 2 | NOT_AVAILABLE_MODE_UNSUITABLE | 3 | NOT_AVAILABLE_SPD_OOR |
| 4 | NOT_AVAILABLE |  |  |  |  |  |  |

#### DriveModes

Java name `com.olaelectric.vehicleinterface.DriveModes`. Host use: yes. Wire: int ordinal (section 2.5). 12 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | PARK | 1 | ECO | 2 | NORMAL | 3 | SPORT |
| 4 | CUSTOM | 5 | REVERSE | 6 | LIMPHOME | 7 | HYPER |
| 8 | ADAPTIVE_BOOST | 9 | SERVICE | 10 | PARK_ASSIST | 11 | BOOST2 |

#### ECUModes

Java name `com.olaelectric.vehicleinterface.ECUModes`. Host use: no. Wire: int ordinal (section 2.5). 8 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | ECU_POWERUP | 2 | DIAGNOSTIC | 3 | AUX_BATTERY_CHARGE |
| 4 | DRIVE_NORMAL | 5 | LIMPHOME | 6 | DRIVE_FAULT | 7 | Reserved |

#### FallDetection

Java name `com.olaelectric.vehicleinterface.FallDetection`. Host use: yes. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | ACTIVE | 2 | INACTIVE | 3 | RECOVERED |

#### HUMessage

Java name `com.olaelectric.vehicleinterface.HUMessage`. Host use: no. Wire: int ordinal (section 2.5). 2 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | No_Request | 1 | CAN_WakeUp |  |  |  |  |

#### HillHoldStatus

Java name `com.olaelectric.vehicleinterface.HillHoldStatus`. Host use: no. Wire: int ordinal (section 2.5). 7 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | UNAVAILABLE | 2 | ACTIVE | 3 | NOT_ACTIVE |
| 4 | ABT_TO_DEACTIVATE | 5 | ABT_TO_ACTIVATE | 6 | HH_ACTIVATED |  |  |

#### HmiStatusBarStt

Java name `com.olaelectric.vehicleinterface.HmiStatusBarStt`. Host use: no. Wire: int ordinal (section 2.5). 6 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | CHARGE_COMPLETE | 2 | CHARGING_SLOWLY | 3 | NOT_CHARGING |
| 4 | INITIALIZING_HYPERCHARGER | 5 | CHECKING_DUES_STATUS |  |  |  |  |

#### IndicatorFault

Java name `com.olaelectric.vehicleinterface.IndicatorFault`. Host use: yes. Wire: int ordinal (section 2.5). 12 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NONE | 1 | TURN_LEFT_INDICATOR | 2 | TURN_RIGHT_INDICATOR | 3 | UNLOCK_INDICATOR |
| 4 | LOCK_INDICATOR | 5 | ALARM | 6 | REVERSE_MODE | 7 | FIND_MY_BIKE |
| 8 | HAZARD | 9 | INDICATOR_ERROR | 10 | SECURITY | 11 | HAZARD_FAULT |

#### LockUnlockReason

Java name `com.olaelectric.vehicleinterface.LockUnlockReason`. Host use: no. Wire: int ordinal (section 2.5). 8 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | INVALID | 1 | SSB_LOCKED | 2 | CAPP_LOCKED | 3 | PROXIMITY_LOCKED |
| 4 | AUTOLOCKED | 5 | SSB_UNLOCKED | 6 | PROXIMITY_UNLOCKED | 7 | CAPP_UNLOCKED |

#### MBMSWakeUpReason

Java name `com.olaelectric.vehicleinterface.MBMSWakeUpReason`. Host use: no. Wire: int ordinal (section 2.5). 8 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | Ready_to_Sleep | 2 | Reserved | 3 | Charger_CAN_Wakeup |
| 4 | IMU_Wakeup | 5 | SoC_Push_Button_Wakeup | 6 | RTC_Wakeup | 7 | EOL_WakeUp |

#### MapCommandAck

Java name `com.olaelectric.vehicleinterface.MapCommandAck`. Host use: no. Wire: int ordinal (section 2.5). 3 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NO_COMMAND | 1 | Success | 2 | Failure |  |  |

#### ModeLimitStatus

Java name `com.olaelectric.vehicleinterface.ModeLimitStatus`. Host use: no. Wire: int ordinal (section 2.5). 3 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NO_RESTRICTION | 1 | ECO_NORMAL_AVAILABLE | 2 | ECO_AVAILABLE |  |  |

#### OnboardNavigation

Java name `com.olaelectric.vehicleinterface.OnboardNavigation`. Host use: no. Wire: int ordinal (section 2.5). 2 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | Not_Present | 1 | Present |  |  |  |  |

#### PowerButtonStatus

Java name `com.olaelectric.vehicleinterface.PowerButtonStatus`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | Not_Pressed | 1 | Normal_Press | 2 | Long_Press | 3 | Reserver |

#### ProximityCommandType

Java name `com.olaelectric.vehicleinterface.ProximityCommandType`. Host use: no. Wire: int ordinal (section 2.5). 65 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | UNKNOWN_CMD | 1 | PROXIMITY_SOFTWARE_VERSION | 2 | MAC_ADDRESS_PROXIMITY_BLE | 3 | DEVICE_DISCOVERABLE |
| 4 | DISCONNECT_CURRENT_PHONE | 5 | DEVICE_CONNECTABLE_BOUND_DEVICE | 6 | CLEAR_ALL_PHONE | 7 | NO_OF_PAIRED_PHONE |
| 8 | CURRENT_PHONE_0_MAC | 9 | CURRENT_PHONE_1_MAC | 10 | CURRENT_PHONE_2_MAC | 11 | CURRENT_PHONE_3_MAC |
| 12 | CURRENT_PHONE_4_MAC | 13 | SUCESSFUL_BOND_CONNECTION | 14 | UNAUTHORIZE_ACCESS_ATTEMP_NO | 15 | PROXIMITY_BLE_PAIRING_RESPONSE_FROM_USER |
| 16 | PARTY_MODE | 17 | OLAMAPS_ACK | 18 | PROFILE_ID | 19 | PROXIMITY_FEATURE |
| 20 | DELETE_PROFILE_ID | 21 | GET_CURRENT_CONNECTED_PROFILE_ID | 22 | HMI_LOGGED_IN_PROFILE_ID | 23 | LOCK_UNLOCK_REASON |
| 24 | PROFILEID_BLE_ACK | 25 | PAIRING_POPUP_ACK | 26 | EXPLICITE_COMMAND_PAIRING | 27 | PROXIMITY_PAIRING_ACK |
| 28 | HILL_HOLD_RQST_Enable_Selected | 29 | HILL_HOLD_RQST_Disable_Selected | 30 | REGEN_TYPE_DEFAULT_Selected | 31 | REGEN_TYPE_CITY_RIDE_EN_Selected |
| 32 | REGEN_TYPE_HIGHWAY_GLIDE_EN_Selected | 33 | SMART_HEAD_LIGHT_DEFAULT_Selected | 34 | SMART_HEAD_LIGHT_LOW_BEAM_EN_Selected | 35 | SMART_HEAD_LIGHT_HIGH_BEAM_EN_Selected |
| 36 | SMART_HEAD_LIGHT_SMART_LIGHT_EN_Selected | 37 | BOOST_MODE_Enable_Selected | 38 | BOOST_MODE_Disable_Selected | 39 | FALL_DETECTION_Enable_Selected |
| 40 | FALL_DETECTION_Disable_Selected | 41 | TOW_TAMPER_DETECTION_Enable_Selected | 42 | TOW_TAMPER_DETECTION_Disable_Selected | 43 | PROXIMITY_LOCK_UNLOCK_Enable_Selected |
| 44 | PROXIMITY_LOCK_UNLOCK_Disable_Selected | 45 | SMART_DRL_LIGHT_Enable_Selected | 46 | SMART_DRL_LIGHT_Disable_Selected | 47 | AUTO_HEADLIGHT_HANDLING_Enable_Selected |
| 48 | AUTO_HEADLIGHT_HANDLING_Disable_Selected | 49 | HAZARD_LIGHT_FEATURE_Enable_Selected | 50 | HAZARD_LIGHT_FEATURE_Disable_Selected | 51 | AUTO_INDICATOR_TURN_OFF_FEATURE_Enable_Selected |
| 52 | AUTO_INDICATOR_TURN_OFF_FEATURE_Disable_Selected | 53 | MOTOR_CUTOFF_DELAY_Enable_Selected | 54 | MOTOR_CUTOFF_DELAY_Disable_Selected | 55 | Hill_Hold_Request_Enable_Selected |
| 56 | Hill_Hold_Request_Disable_Selected | 57 | Vacation_Mode_ON | 58 | Vacation_Mode_OFF | 59 | Need_Help |
| 60 | I_am_Safe | 61 | Delete_Profile | 62 | User_Calibration_Request | 63 | TMH_Lights_Enable |
| 64 | TMH_Lights_Disabled |  |  |  |  |  |  |

#### RegenAvailability

Java name `com.olaelectric.vehicleinterface.RegenAvailability`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | REGEN_NOT_AVAILABLE | 1 | REGEN_AVAILABLE | 2 | REGEN_NOT_AVAILABLE_THROTTLE | 3 | Reserved |

#### RegenStatus

Java name `com.olaelectric.vehicleinterface.RegenStatus`. Host use: yes. Wire: int ordinal (section 2.5). 5 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | Not_Active | 1 | Coast_Regen | 2 | Coast_Forced_Regen | 3 | Coast_Brake_Regen |
| 4 | Coast_Forced_Brake_Regen |  |  |  |  |  |  |

#### SomState

Java name `com.olaelectric.vehicleinterface.SomState`. Host use: no. Wire: int ordinal (section 2.5). 7 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | OFF | 1 | BOOTING | 2 | SLEEP | 3 | STAND_BY |
| 4 | FASTBOOT | 5 | KEY_OFF | 6 | KEY_ON |  |  |

#### TowTheftDetection

Java name `com.olaelectric.vehicleinterface.TowTheftDetection`. Host use: yes. Wire: int ordinal (section 2.5). 3 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | ACTIVE | 2 | INACTIVE |  |  |

#### VehicleLockStateBCM

Java name `com.olaelectric.vehicleinterface.VehicleLockStateBCM`. Host use: yes. Wire: int ordinal (section 2.5). 5 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | Vehicle_Unlocked | 2 | Vehicle_Unlock_Fail | 3 | Vehicle_Locked |
| 4 | Vehicle_Lock_Fail |  |  |  |  |  |  |

#### VehicleMonitorMode

Java name `com.olaelectric.vehicleinterface.VehicleMonitorMode`. Host use: no. Wire: int ordinal (section 2.5). 4 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NOT_MAPPED | 1 | IN_PURCHASE | 2 | SERVICE_JOB_OPEN | 3 | SERVICE_JOB_CLOSED |

#### VehicleSafetyAlerts

Java name `com.olaelectric.vehicleinterface.VehicleSafetyAlerts`. Host use: no. Wire: int ordinal (section 2.5). 13 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | No_Command | 1 | Battery_Temperature_High_Alert | 2 | Battery_Temperature_Very_High_Alert | 3 | Battery_Temperature_Low_Alert |
| 4 | Battery_Temperature_Very_Low_Alert | 5 | Motor_Temperature_High_Alert | 6 | Motor_Temperature_Very_High_Alert | 7 | Motor_Temperature_Low_Alert |
| 8 | Motor_Temperature_Very_Low_Alert | 9 | Side_Stand_Position_Alert | 10 | Steering_Lock_Unlock_Alert | 11 | Tamper_Alert |
| 12 | Current_Drive_Mode |  |  |  |  |  |  |

#### VehicleStates

Java name `com.olaelectric.vehicleinterface.VehicleStates`. Host use: yes. Wire: int ordinal (section 2.5). 16 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | SNA | 1 | BC_CHARGE_ACTIVE | 2 | VACATION | 3 | SLEEP |
| 4 | VEHICLE_IDLE | 5 | SC_CHARGE_IDLE | 6 | SC_CHARGE_FAULT | 7 | SC_CHARGE_ACTIVE |
| 8 | FC_CHARGE_IDLE | 9 | FC_CHARGE_FAULT | 10 | FC_CHARGE_ACTIVE | 11 | PARK |
| 12 | DRIVE | 13 | ALARM | 14 | BC_CHARGE_IDLE | 15 | BC_CHARGE_FAULT |

#### VehicleTypes

Java name `com.olaelectric.vehicleinterface.VehicleTypes`. Host use: no. Wire: int ordinal (section 2.5). 17 constants.

| Int | Constant | Int | Constant | Int | Constant | Int | Constant |
|---|---|---|---|---|---|---|---|
| 0 | NA | 1 | S1 | 2 | S1_Pro_wo_upgrade | 3 | S1_Pro_with_upgrade |
| 4 | S1_2Kw | 5 | S1_Air | 6 | S1_Pro_Gen2_With_Upgrade | 7 | S1X_Plus |
| 8 | S1_Air_Fleet | 9 | S1_Pro_Gen2_WO_Upgrade | 10 | S1XPlus_Phase2 | 11 | Reserved |
| 12 | S1XPlus_Phase1_4KwH | 13 | S1XPlus_Phase2_4KwH | 14 | S1X_2KWh | 15 | S1X_3KWh |
| 16 | S1X_4KWh |  |  |  |  |  |  |


## 7. Parcelables

Three parcelable types exist. Each is written with its fields in the order shown, with no header
or length, and read back in the same order. The enclosing argument is nullable (section 2.4).

### 7.1 BleDetails

| Order | Field | Wire type |
|---|---|---|
| 1 | proximityBleAddress | string |
| 2 | proximityCommunicationService | string |
| 3 | proximityTxCharacteristics | string |
| 4 | proximityRxCharacteristics | string |

### 7.2 IMUData

| Order | Field | Wire type |
|---|---|---|
| 1 | accelerometer | float |
| 2 | gyrometer | float |

### 7.3 VehicleRangeBasedOnModes

| Order | Field | Wire type |
|---|---|---|
| 1 | Vehicle_Range_Eco | int |
| 2 | Vehicle_Range_Sport | int |
| 3 | Vehicle_Range_Normal | int |
| 4 | Vehicle_Range_Hyper | int |

## 8. Implementer notes and ambiguities

1. Descriptor. The binder checks the interface token against the descriptor, so every interface must
   use the exact descriptor string in section 4. The Java package used to generate the stub must
   produce that string. If a different Java package is used, the descriptor string must be set to
   the value in section 4 explicitly.
2. Method order. In the replacement interfaces, methods must be declared in an order that produces
   the codes in section 5. The listed codes govern where they differ from declaration order.
   Methods the host does not call are still required so that later codes stay correct.
3. Names are load-bearing. Type names, method names and enumeration constant names must be reproduced
   exactly as listed, in addition to the positions. The host refers to constants and methods by these
   names, and its logs record them. Only parameter names are free. Constant order in section 6 must
   match exactly, including unusual spellings.
4. Two-way callbacks. The callbacks of IVehicleServiceInfoCallbacks are two-way. The receiver must
   reply with int 0 after dispatching. The other 14 callback interfaces are oneway and must not
   reply.
5. Enumerations, not parcelables. IndicatorFault and HUMessage are enumerations, not parcelables.
   They are written like every other enumeration (nullable presence int, then index).
6. Unlock path. The host's unlock flow does not call ICommanderService. It tracks state through the
   callbacks onVehicleLockStateBCMChanged, onVehicleStateChanged and onPasscodeWindowActivated, and
   performs the passcode check through the separate passcode manager (section 9). The host must not
   call unlockedByPasscode (code 1) or send PASSCODE_AUTH through sendCommandToScooter, because that
   would be a second unlock request.
7. Booleans in commander parameter lists. sendPartyModeCmd takes two booleans and an int; both
   booleans are written as int 1 or 0 in order. sendHyperChargerDueStatus takes a nullable
   enumeration and then a boolean. Both follow the rules in section 5.3.
8. Parameter names (the Java identifiers of arguments) are not on the wire and may differ.
9. Cross-check. Every callback method name seen in the ride logs of 2026-10-30 (tmp/rl2c) appears in
   section 5.4. Every enumeration constant name seen in those logs appears in section 6. Argument types
   were not compared against the logs.
10. AIDL declarations. The 37 enumerations and 3 parcelables must be declared in AIDL as parcelable
    types, not with the AIDL enum keyword. The AIDL enum keyword writes a bare int with no presence
    marker, which does not match section 2.4. Each enumeration is a Java enum implementing Parcelable:
    writeToParcel writes the int index, and the CREATOR reads an int and maps it by index (section
    2.5). Each parcelable class writes its fields in the order of section 7, and its CREATOR reads them
    back in the same order. AIDL generates the nullable presence int for parcelable-typed arguments.
11. Versioning. No version is exchanged on the wire. A replacement interoperates only if its codes and
    signatures match this document, which reflects the stock vehicle-parser service in the platform
    version this specification was derived from.

## 9. Not covered

- The passcode/profile manager service used by the unlock path. It has a separate descriptor and
  separate transactions and needs its own specification.
- Any method, callback or enumeration not listed above.
- Internal behaviour of the parser service, its timing, and the meaning of individual values beyond
  the names in the enumeration tables.
