### TiDC

TiDC is a simple, minimal decoder for the [unified log format](https://github.com/tikv/rfcs/blob/master/text/0018-unified-log-format.md) of the TiKV community.

#### Usage

Yep, there isn't any command line arguments supported(for now).

It only drains logs from stdin, then transforms them into json and send to stdout.

Also no release or version number for now. 
You may need to compile and install it manually (say, `cargo install --path .`) until the author get ready, sorry for that.

```bash
echo '[2018/12/15 14:20:11.015 +08:00] [INFO] [tikv-server.rs:13] ["TiKV Started"]' | tidc
# stdout: 
# {"message":"TiKV Started","level":"info","source":{"file":"tikv-server.rs","line":"13"},"time":"2018/12/15 14:20:11.015 +08:00","fields":{}}
tidc <<EOF
[2018/12/15 14:20:11.015 +08:00] [WARN] [session.go:1234] ["Slow query"] [sql="SELECT * FROM TABLE\nWHERE ID=\"abc\""] [duration=1.345s] [client=192.168.0.123:12345] [txn_id=123000102231]
[2018/12/15 14:20:11.015 +08:00] [FATAL] [panic_hook.rs:45] ["TiKV panic"] [stack="   0: std::sys::imp::backtrace::tracing::imp::unwind_backtrace\n             at /checkout/src/libstd/sys/unix/backtrace/tracing/gcc_s.rs:49\n   1: std::sys_common::backtrace::_print\n             at /checkout/src/libstd/sys_common/backtrace.rs:71\n   2: std::panicking::default_hook::{{closure}}\n             at /checkout/src/libstd/sys_common/backtrace.rs:60\n             at /checkout/src/libstd/panicking.rs:381"] [error="thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 99"]
EOF
# stdout:
# { "message":"Slow query","level":"warn","source":{"file":"session.go","line":"1234"},"time":"2018/12/15 14:20:11.015 +08:00","fields": {"sql":"SELECT * FROM TABLE\nWHERE ID=\"abc\"","duration":"1.345s","client":"192.168.0.123:12345","txn_id":"123000102231"}}
# {"message":"TiKV panic","level":"fatal","source":{"file":"panic_hook.rs","line":"45"},"time":"2018/12/15 14:20:11.015 +08:00","fields":{"stack":"   0: std::sys::imp::backtrace::tracing::imp::unwind_backtrace\n             at /checkout/src/libstd/sys/unix/backtrace/tracing/gcc_s.rs:49\n   1: std::sys_common::backtrace::_print\n             at /checkout/src/libstd/sys_common/backtrace.rs:71\n   2: std::panicking::default_hook::{{closure}}\n             at /checkout/src/libstd/sys_common/backtrace.rs:60\n             at /checkout/src/libstd/panicking.rs:381","error":"thread 'main' panicked at 'index out of bounds: the len is 3 but the index is 99"}}
```

Then, use the fantastic tool `jq` to analyze some logs(no more `awk -f'['` like things...):

```bash
cat somewhat-backup.log | ./target/release/tidc | jq 'select(.message | test("backup streaming finish")) | .fields.StoreID' | sort | uniq -c
# stdout:
# 1473 "1"
# 1473 "4"
# 1473 "5"
# 1473 "540"
# 1473 "541"
# 1473 "542"
# 1473 "543"
```

Eh, maybe in some way, this is style of the UNIX: compose simple programs can do amazing things, I guess?
