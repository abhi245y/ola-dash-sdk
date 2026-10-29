package com.olaelectric.vehicleinterface;

import android.os.Parcel;
import android.os.Parcelable;

/** Range per driving mode (SPEC 7.3). Written as four ints in this order. */
public final class VehicleRangeBasedOnModes implements Parcelable {
    public final int rangeEco;
    public final int rangeSport;
    public final int rangeNormal;
    public final int rangeHyper;

    public VehicleRangeBasedOnModes(int eco, int sport, int normal, int hyper) {
        this.rangeEco = eco;
        this.rangeSport = sport;
        this.rangeNormal = normal;
        this.rangeHyper = hyper;
    }

    @Override
    public int describeContents() {
        return 0;
    }

    @Override
    public void writeToParcel(Parcel out, int flags) {
        out.writeInt(rangeEco);
        out.writeInt(rangeSport);
        out.writeInt(rangeNormal);
        out.writeInt(rangeHyper);
    }

    public static final Parcelable.Creator<VehicleRangeBasedOnModes> CREATOR =
            new Parcelable.Creator<VehicleRangeBasedOnModes>() {
        @Override
        public VehicleRangeBasedOnModes createFromParcel(Parcel in) {
            return new VehicleRangeBasedOnModes(in.readInt(), in.readInt(), in.readInt(), in.readInt());
        }

        @Override
        public VehicleRangeBasedOnModes[] newArray(int size) {
            return new VehicleRangeBasedOnModes[size];
        }
    };

    @Override
    public String toString() {
        return "VehicleRangeBasedOnModes[" + rangeEco + ", " + rangeSport + ", "
                + rangeNormal + ", " + rangeHyper + "]";
    }
}
