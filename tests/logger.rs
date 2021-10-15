use slog::{Fuse, Logger};
use tezedge_actor_system::actors::*;

mod common {
    use std::{fmt, result};

    use slog::{Drain, OwnedKVList, Record, KV};

    pub struct PrintlnSerializer;

    impl slog::Serializer for PrintlnSerializer {
        fn emit_arguments(&mut self, key: slog::Key, val: &fmt::Arguments) -> slog::Result {
            print!(", {}={}", key, val);
            Ok(())
        }
    }

    pub struct PrintlnDrain;

    impl Drain for PrintlnDrain {
        type Ok = ();
        type Err = ();

        fn log(
            &self,
            record: &Record,
            values: &OwnedKVList,
        ) -> result::Result<Self::Ok, Self::Err> {
            print!("{}", record.msg());

            record
                .kv()
                .serialize(record, &mut PrintlnSerializer)
                .unwrap();
            values.serialize(record, &mut PrintlnSerializer).unwrap();

            println!();
            Ok(())
        }
    }
}

#[test]
fn system_create_with_slog() {
    let log = Logger::root(
        Fuse(common::PrintlnDrain),
        slog::o!("version" => "v1", "run_env" => "test"),
    );
    let sys = ActorSystem::new().unwrap();
    sys.shutdown();
    let _ = log;
}

// a test that logging without slog using "log" crate works
#[test]
fn logging_stdlog() {
    log::info!("before the system");
    let _sys = ActorSystem::new().unwrap();
    log::info!("system exists");
}
