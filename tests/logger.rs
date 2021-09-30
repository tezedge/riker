use riker::actors::*;
use slog::{o, Fuse, Logger};

mod common {
    use std::{fmt, result};

    use slog::*;

    pub struct PrintlnSerializer;

    impl Serializer for PrintlnSerializer {
        fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> Result {
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

#[tokio::test]
async fn system_create_with_slog() {
    let log = Logger::root(
        Fuse(common::PrintlnDrain),
        o!("version" => "v1", "run_env" => "test"),
    );
    let backend = tokio::runtime::Handle::current().into();
    let sys = ActorSystem::new(backend).unwrap();
    sys.shutdown().await;
    let _ = log;
}

// a test that logging without slog using "log" crate works
#[tokio::test]
async fn logging_stdlog() {
    log::info!("before the system");
    let backend = tokio::runtime::Handle::current().into();
    let _sys = ActorSystem::new(backend).unwrap();
    log::info!("system exists");
}
