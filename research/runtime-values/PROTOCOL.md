# Runtime value storage, v1

Generic IL2CPP metadata extension. Read `class_is_enum`, `class_enum_basetype`,
`class_value_size`, and the existing runtime field/type/header APIs outside GC
pauses. These exports are optional as a group: older providers keep scalar/reference
support. No game names, offsets, field names or source layouts enter the product.

Use runtime value size and boxed field offset minus runtime object header for
embedded storage. Inline nodes are keyed by address, declared class and storage
kind; an inline object never goes to `object_get_class`. Scalar enum reads retain
their declared type in the containing layout digest. References inside structures
retain normal object semantics. A readonly enclosing structure prevents leaf
writes until an actual object reference is followed. Static primitive/enum roots,
enum label interpretation, pointers/byrefs and recursive/invalid layouts remain
unsupported. Discovery retains the existing depth/node/read/time budgets.

The owned native ABI fixture must establish nested zero-offset structures,
closed generic value types, arrays of structures and enums, byte and exact u64
enum values, static structures, bounds/null/layout rejection, readonly enclosing
fields, successful explicit scalar change and restoration, zero helper Rust heap
operations during every managed pause and balanced stop/start calls. The fixture
aborts if metadata APIs run paused or object APIs inspect unboxed data. Test an
older export provider as a compatibility check. Freeze source/binary hashes and
retain failures separately; do not replace old evidence.

After fixture success, use the unchanged reader on an independently implemented
installed game in Anode. Record catalog coverage, embedded/scalar counts and fresh
path reads without semantic naming or gameplay writes. This measures reachability,
not meaningful control discovery or generalization. No model calls are needed.

API/offset evidence: installed Unity 2022.3.18f1 SDK's `il2cpp-api-functions.h`,
`vm/Class.cpp` (`GetValueSize`), `vm/Field.cpp` (`GetInstanceFieldDataPointer`).
No independently implemented game's source or decompiled field layout is used.
