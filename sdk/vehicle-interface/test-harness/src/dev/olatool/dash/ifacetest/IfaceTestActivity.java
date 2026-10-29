package dev.olatool.dash.ifacetest;

import android.app.Activity;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.os.Bundle;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.util.Log;

import com.olaelectric.vehicleinterface.AlertInfoCallbacksBase;
import com.olaelectric.vehicleinterface.BatteryInfoCallbacksBase;
import com.olaelectric.vehicleinterface.ChargerInfoCallbacksBase;
import com.olaelectric.vehicleinterface.DisplayInfoCallbacksBase;
import com.olaelectric.vehicleinterface.HandleBarCallbacksBase;
import com.olaelectric.vehicleinterface.HardwareVersionInfoCallbacksBase;
import com.olaelectric.vehicleinterface.IParserService;
import com.olaelectric.vehicleinterface.IVehicleService;
import com.olaelectric.vehicleinterface.ImuInfoCallbacksBase;
import com.olaelectric.vehicleinterface.MapInfoCallbacksBase;
import com.olaelectric.vehicleinterface.MotorInfoCallbacksBase;
import com.olaelectric.vehicleinterface.ProximityCallbacksBase;
import com.olaelectric.vehicleinterface.VehicleCommandCallbacksBase;
import com.olaelectric.vehicleinterface.VehicleInfoCallbacksBase;
import com.olaelectric.vehicleinterface.VehicleModeCallbacksBase;
import com.olaelectric.vehicleinterface.VehicleServiceInfoCallbacksBase;
import com.olaelectric.vehicleinterface.WidgetInfoCallbacksBase;

import dev.olatool.dash.Trace;

/**
 * Binds the vehicle-parser service, subscribes all 15 callback interfaces with logging
 * bases, then finishes after RUN_MS. Requests only the parser binder: the commander
 * binder is never obtained or called.
 */
public class IfaceTestActivity extends Activity {
    private static final String TAG = "IfaceTest";
    private static final String PARSER_PKG = "com.olaelectric.vehicleparser";
    private static final String PARSER_CLASS = "com.olaelectric.vehicleparser.services.VehicleParserService";
    private static final String PARSER_ACTION = "com.olaelectric.vehicleparser.VehicleParserService";
    private static final long RUN_MS = 30_000;

    private final Handler main = new Handler(Looper.getMainLooper());
    private ServiceConnection conn;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        Trace.init(this);
        Log.i(TAG, "start key=" + getPackageName());
        conn = new ServiceConnection() {
            @Override
            public void onServiceConnected(ComponentName name, IBinder service) {
                new Thread(() -> subscribeAll(service), "ifacetest-subscribe").start();
            }

            @Override
            public void onServiceDisconnected(ComponentName name) {
                Log.w(TAG, "parser disconnected");
            }
        };
        Intent i = new Intent(PARSER_ACTION).setComponent(new ComponentName(PARSER_PKG, PARSER_CLASS));
        Log.i(TAG, "bind=" + bindService(i, conn, Context.BIND_AUTO_CREATE));
        main.postDelayed(this::finish, RUN_MS);
    }

    private void subscribeAll(IBinder vehicleBinder) {
        try {
            IVehicleService vehicle = IVehicleService.Stub.asInterface(vehicleBinder);
            IParserService parser = vehicle.getParserServiceBinder();
            // The host uses its package name as the subscription key. This harness uses its own,
            // so it does not replace the host's subscriptions.
            String key = getPackageName();
            log("subscribeDisplayInfo", parser.subscribeDisplayInfo(key, new DisplayInfoCallbacksBase()));
            log("subscribeMotorInfo", parser.subscribeMotorInfo(key, new MotorInfoCallbacksBase()));
            log("subscribeVehicleModes", parser.subscribeVehicleModes(key, new VehicleModeCallbacksBase()));
            log("subscribeBatteryInfo", parser.subscribeBatteryInfo(key, new BatteryInfoCallbacksBase()));
            log("subscribeHandleBar", parser.subscribeHandleBar(key, new HandleBarCallbacksBase()));
            log("subscribeAlertInfo", parser.subscribeAlertInfo(key, new AlertInfoCallbacksBase()));
            log("subscribeIMUInfo", parser.subscribeIMUInfo(key, new ImuInfoCallbacksBase()));
            log("subscribeVehicleCommand", parser.subscribeVehicleCommand(key, new VehicleCommandCallbacksBase()));
            log("subscribeProximityInfo", parser.subscribeProximityInfo(key, new ProximityCallbacksBase()));
            log("subscribeHardwareVersionInfo",
                    parser.subscribeHardwareVersionInfo(key, new HardwareVersionInfoCallbacksBase()));
            log("subscribeWidgetData", parser.subscribeWidgetData(key, new WidgetInfoCallbacksBase()));
            log("subscribeVehicleInfo", parser.subscribeVehicleInfo(key, new VehicleInfoCallbacksBase()));
            log("subscribeMapInfo", parser.subscribeMapInfo(key, new MapInfoCallbacksBase()));
            log("subscribeChargerInfo", parser.subscribeChargerInfo(key, new ChargerInfoCallbacksBase()));
            log("subscribeVehicleServiceInfo",
                    parser.subscribeVehicleServiceInfo(key, new VehicleServiceInfoCallbacksBase()));
        } catch (Exception e) {
            Log.e(TAG, "subscribe failed", e);
        }
    }

    private static void log(String method, boolean ok) {
        Trace.call("subscribe", method, ok);
    }

    @Override
    protected void onDestroy() {
        if (conn != null) {
            unbindService(conn);
        }
        super.onDestroy();
    }
}
