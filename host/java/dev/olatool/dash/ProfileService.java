package dev.olatool.dash;

import android.os.IBinder;
import android.os.Parcel;
import android.os.RemoteException;

/**
 * Small, wire-compatible client for the three PMS methods needed by the keypad.
 * Source: HHMIC com/olaelectric/pmservice/IProfileManagerService.java (transactions
 * 1, 5, 21) and PasscodeStatus.java (int attempts, byte status). No PIN is persisted.
 * challengeUnlockByPasscode(pin, false) performs stock authentication AND requests
 * unlock. Never follow it with a direct commander unlock or PASSCODE_AUTH command.
 */
final class ProfileService {
    static final String PACKAGE = "com.olaelectric.profilemanagerservice";
    static final String SERVICE = PACKAGE + ".services.ProfileManagerService";
    private static final String DESCRIPTOR = "com.olaelectric.pmservice.IProfileManagerService";
    private final IBinder binder;

    static final class Result {
        final int attempts;
        final boolean valid;
        Result(int attempts, boolean valid) { this.attempts = attempts; this.valid = valid; }
    }

    ProfileService(IBinder binder) { this.binder = binder; }

    Result unlock(String pin) throws RemoteException {
        Parcel request = Parcel.obtain(), response = Parcel.obtain();
        try {
            request.writeInterfaceToken(DESCRIPTOR);
            request.writeString(pin);
            request.writeInt(0); // false: normal unlock, not the change-passcode screen
            transact(1, request, response);
            if (response.readInt() == 0) throw new RemoteException("Missing passcode result");
            return new Result(response.readInt(), response.readByte() != 0);
        } finally {
            response.recycle();
            request.recycle();
        }
    }

    int remainingAttempts() throws RemoteException {
        Parcel request = Parcel.obtain(), response = Parcel.obtain();
        try {
            request.writeInterfaceToken(DESCRIPTOR);
            transact(5, request, response);
            return response.readInt();
        } finally {
            response.recycle(); request.recycle();
        }
    }

    long firstTryTimestamp() throws RemoteException {
        Parcel request = Parcel.obtain(), response = Parcel.obtain();
        try {
            request.writeInterfaceToken(DESCRIPTOR);
            transact(21, request, response);
            return response.readLong();
        } finally {
            response.recycle(); request.recycle();
        }
    }

    private void transact(int code, Parcel request, Parcel response) throws RemoteException {
        if (!binder.transact(code, request, response, 0)) {
            throw new RemoteException("Unsupported profile service transaction " + code);
        }
        response.readException();
    }
}
