package com.olaelectric.vehicleinterface;

/** Index lookup for enumeration reads (SPEC 2.5). An index outside the constants fails the read. */
final class Wire {
    private Wire() {
    }

    static <E> E at(E[] constants, int index) {
        if (index < 0 || index >= constants.length) {
            throw new IllegalArgumentException("enum index " + index + " out of range");
        }
        return constants[index];
    }
}
