# Vehicle-interface SDK surface (clean-room)

`SPEC.md` in this directory is an interoperability specification of the binder protocol
between the SDK host and the vehicle-parser service on the Ola S1 Pro Gen 1. It lists the
interfaces, transaction codes, argument order, enumeration integers and parcelable fields
that the host uses or must receive.

The specification was derived from interoperability analysis: it records names, order,
types and integers only. It contains no source text, comments or implementation logic from
any vendor code.

Clean-room process for the replacement interface:

1. The author of `SPEC.md` works from the analysis material. No `.aidl` or `.java` files for
   this interface are produced at that step.
2. The implementer writes the replacement `.aidl` files and enumerations from `SPEC.md` alone.
   The implementer must not open the vendor-derived decompiled sources (they are git-ignored
   and never published).

Licence: GPL-3.0-or-later, as the rest of the SDK.
