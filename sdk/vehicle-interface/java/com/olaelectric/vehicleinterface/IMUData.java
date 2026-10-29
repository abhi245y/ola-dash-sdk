package com.olaelectric.vehicleinterface;

import android.os.Parcel;
import android.os.Parcelable;

/** IMU sample (SPEC 7.2). Written as two floats in this order. */
public final class IMUData implements Parcelable {
    public final float accelerometer;
    public final float gyrometer;

    public IMUData(float accelerometer, float gyrometer) {
        this.accelerometer = accelerometer;
        this.gyrometer = gyrometer;
    }

    @Override
    public int describeContents() {
        return 0;
    }

    @Override
    public void writeToParcel(Parcel out, int flags) {
        out.writeFloat(accelerometer);
        out.writeFloat(gyrometer);
    }

    public static final Parcelable.Creator<IMUData> CREATOR = new Parcelable.Creator<IMUData>() {
        @Override
        public IMUData createFromParcel(Parcel in) {
            return new IMUData(in.readFloat(), in.readFloat());
        }

        @Override
        public IMUData[] newArray(int size) {
            return new IMUData[size];
        }
    };

    @Override
    public String toString() {
        return "IMUData[" + accelerometer + ", " + gyrometer + "]";
    }
}
