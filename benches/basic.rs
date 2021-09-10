#![forbid(unsafe_code)]

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rand::Rng;
use riker::actors::*;

struct WorkerActor {
    main: ActorRef<MainMessage>,
    done: Arc<AtomicUsize>,
}

#[derive(Debug, Clone)]
enum WorkerMessage {
    Data(Vec<u8>),
}

impl ActorFactoryArgs<(ActorRef<MainMessage>, Arc<AtomicUsize>)> for WorkerActor {
    fn create_args((main, done): (ActorRef<MainMessage>, Arc<AtomicUsize>)) -> Self {
        WorkerActor {
            main,
            done,
        }
    }
}

impl Actor for WorkerActor {
    type Msg = WorkerMessage;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, _sender: Sender) {
        match msg {
            WorkerMessage::Data(data) => {
                self.main.tell(MainMessage::Data(data), Some(ctx.myself().into()));
            }
        }
    }

    fn post_stop(&mut self) {
        self.done.fetch_add(1, Ordering::SeqCst);
    }
}

struct MainActor {
    done: Arc<AtomicUsize>,
    worker_counter: usize,
}

#[derive(Debug, Clone)]
enum MainMessage {
    CreateActor(Vec<u8>),
    Data(Vec<u8>),
}

impl ActorFactoryArgs<Arc<AtomicUsize>> for MainActor {
    fn create_args(done: Arc<AtomicUsize>) -> Self {
        MainActor {
            done,
            worker_counter: 0,
        }
    }
}

impl Actor for MainActor {
    type Msg = MainMessage;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        match msg {
            MainMessage::CreateActor(data) => {
                let name = format!("worker-{}", self.worker_counter);
                let worker = ctx.system
                    .actor_of_args::<WorkerActor, _>(&name, (ctx.myself(), self.done.clone()))
                    .unwrap();
                self.worker_counter += 1;
                worker.tell(WorkerMessage::Data(data), Some(ctx.myself().into()));
            },
            MainMessage::Data(data) => {
                if let Some(worker) = sender {
                    ctx.system.stop(&worker);
                } else {
                    self.done.fetch_add(1, Ordering::SeqCst);
                }
                black_box(data);
            },
        }
    }
}

fn single_actor(c: &mut Criterion) {
    const MESSAGES: usize = 0x100;

    let data = (0..MESSAGES).map(|_| {
        let mut data = vec![0; 0x1000];
        rand::thread_rng().fill(data.as_mut_slice());
        data    
    }).collect::<Vec<_>>();
    let sys = ActorSystem::new().unwrap();
    let done = Arc::new(AtomicUsize::new(0));
    let main_actor = sys.actor_of_args::<MainActor, _>("main-actor", done.clone()).unwrap();

    c.bench_function("single_actor", |b| {
        b.iter(|| {
            for data in &data {
                main_actor.tell(black_box(MainMessage::Data(data.clone())), None);
            }
            while done.load(Ordering::SeqCst) < MESSAGES {
                std::thread::yield_now();
            }
            done.store(0, Ordering::SeqCst);
        })
    });
}

fn multiple_actors(c: &mut Criterion) {
    const MESSAGES: usize = 0x100;

    let data = (0..MESSAGES).map(|_| {
        let mut data = vec![0; 0x1000];
        rand::thread_rng().fill(data.as_mut_slice());
        data    
    }).collect::<Vec<_>>();
    let sys = ActorSystem::new().unwrap();
    let done = Arc::new(AtomicUsize::new(0));
    let main_actor = sys.actor_of_args::<MainActor, _>("main-actor", done.clone()).unwrap();

    c.bench_function("multiple_actors", |b| {
        b.iter(|| {
            for data in &data {
                main_actor.tell(black_box(MainMessage::CreateActor(data.clone())), None);
            }
            while done.load(Ordering::SeqCst) < MESSAGES {
                std::thread::yield_now();
            }
            done.store(0, Ordering::SeqCst);
        })
    });
}

criterion_group!(basic, single_actor, multiple_actors);
criterion_main!(basic);
