package com.olaelectric.vehicleinterface;

import android.os.Parcel;
import android.os.Parcelable;

/** Proximity BLE details (SPEC 7.1). Written as four strings in this order. */
public final class BleDetails implements Parcelable {
    public final String proximityBleAddress;
    public final String proximityCommunicationService;
    public final String proximityTxCharacteristics;
    public final String proximityRxCharacteristics;

    public BleDetails(String bleAddress, String communicationService,
                      String txCharacteristics, String rxCharacteristics) {
        this.proximityBleAddress = bleAddress;
        this.proximityCommunicationService = communicationService;
        this.proximityTxCharacteristics = txCharacteristics;
        this.proximityRxCharacteristics = rxCharacteristics;
    }

    @Override
    public int describeContents() {
        return 0;
    }

    @Override
    public void writeToParcel(Parcel out, int flags) {
        out.writeString(proximityBleAddress);
        out.writeString(proximityCommunicationService);
        out.writeString(proximityTxCharacteristics);
        out.writeString(proximityRxCharacteristics);
    }

    public static final Parcelable.Creator<BleDetails> CREATOR = new Parcelable.Creator<BleDetails>() {
        @Override
        public BleDetails createFromParcel(Parcel in) {
            return new BleDetails(in.readString(), in.readString(), in.readString(), in.readString());
        }

        @Override
        public BleDetails[] newArray(int size) {
            return new BleDetails[size];
        }
    };

    @Override
    public String toString() {
        return "BleDetails[" + proximityBleAddress + ", " + proximityCommunicationService + ", "
                + proximityTxCharacteristics + ", " + proximityRxCharacteristics + "]";
    }
}
