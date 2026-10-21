# Native verification of already-original Undo

Freeze the release binaries, unchanged owned ABI fixture, runner and host hashes
before starting. Through the public MCP engine: write a numeric and a boolean
value, independently return each to its original bytes in the owned host, and
request Undo once. Require `already_original`, restored phase, exact original
bytes from independent host reads, and native attempted=false/transferred=0.
Repeat each request and require the identical saved trial with no helper call.

Negative controls: a different current number, and original bytes with a replaced
reference chain, must remain refused. Never relabel these as restored. Restart
the idle owned editor and require exactly preserved history and replay without
native operations. Four trials, six independent byte observations, no gameplay,
input or screenshots. Audit every helper completion/unload and allocation count;
close all owned children. Native failure remains evidence, never a reason to
retry the same effect blindly. Unit checks separately vary scope, type, address,
reference chain, bytes/value, guard and write-attempt receipts, and preserve the
old refused-history interpretation.
