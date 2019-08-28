#[macro_use]
extern crate riker_testkit;

use async_trait::async_trait;
use futures::executor::block_on;

use riker::actors::*;

use riker_testkit::probe::channel::{probe, ChannelProbe};
use riker_testkit::probe::{Probe, ProbeReceive};

#[derive(Clone, Debug)]
pub struct TestProbe(ChannelProbe<(), ()>);

// a simple minimal actor for use in tests
// #[actor(TestProbe)]
struct Child;

impl Child {
    fn new() -> Self {
        Child
    }
}

#[async_trait]
impl Actor for Child {
    type Msg = TestProbe;

    async fn recv(&mut self, _ctx: &Context<Self::Msg>, msg: Self::Msg, _sender: Sender) {
        msg.0.event(());
    }
}

struct SelectTest;

impl SelectTest {
    fn new() -> Self {
        SelectTest
    }
}

#[async_trait]
impl Actor for SelectTest {
    type Msg = TestProbe;

    async fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        // create first child actor
        let props = Props::new(Child::new);
        let _ = ctx.actor_of(props, "child_a").await.unwrap();

        // create second child actor
        let props = Props::new(Child::new);
        let _ = ctx.actor_of(props, "child_b").await.unwrap();
    }

    async fn recv(&mut self, _ctx: &Context<Self::Msg>, msg: Self::Msg, _sender: Sender) {
        msg.0.event(());
    }
}

#[test]
fn select_child() {
    let sys = block_on(ActorSystem::new()).unwrap();

    let props = Props::new(SelectTest::new);
    block_on(sys.actor_of(props, "select-actor")).unwrap();

    let (probe, listen) = probe();

    // select test actors through actor selection: /root/user/select-actor/*
    let sel = sys.select("select-actor").unwrap();

    sel.try_tell(TestProbe(probe), None);

    p_assert_eq!(listen, ());
}

#[test]
fn select_child_of_child() {
    let sys = block_on(ActorSystem::new()).unwrap();

    let props = Props::new(SelectTest::new);
    block_on(sys.actor_of(props, "select-actor")).unwrap();

    // delay to allow 'select-actor' pre_start to create 'child_a' and 'child_b'
    // Direct messaging on the actor_ref doesn't have this same issue
    std::thread::sleep(std::time::Duration::from_millis(500));

    let (probe, listen) = probe();

    // select test actors through actor selection: /root/user/select-actor/*
    let sel = sys.select("select-actor/child_a").unwrap();
    sel.try_tell(TestProbe(probe), None);

    // actors 'child_a' should fire a probe event
    p_assert_eq!(listen, ());
}

#[test]
fn select_all_children_of_child() {
    let sys = block_on(ActorSystem::new()).unwrap();

    let props = Props::new(SelectTest::new);
    block_on(sys.actor_of(props, "select-actor")).unwrap();

    // delay to allow 'select-actor' pre_start to create 'child_a' and 'child_b'
    // Direct messaging on the actor_ref doesn't have this same issue
    std::thread::sleep(std::time::Duration::from_millis(500));

    let (probe, listen) = probe();

    // select relative test actors through actor selection: /root/user/select-actor/*
    let sel = sys.select("select-actor/*").unwrap();
    sel.try_tell(TestProbe(probe.clone()), None);

    // actors 'child_a' and 'child_b' should both fire a probe event
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());

    // select absolute test actors through actor selection: /root/user/select-actor/*
    let sel = sys.select("/user/select-actor/*").unwrap();
    sel.try_tell(TestProbe(probe), None);

    // actors 'child_a' and 'child_b' should both fire a probe event
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
}

#[derive(Clone)]
struct SelectTest2;

impl SelectTest2 {
    fn new() -> Self {
        SelectTest2
    }
}

#[async_trait]
impl Actor for SelectTest2 {
    type Msg = TestProbe;

    async fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        // create first child actor
        let props = Props::new(Child::new);
        let _ = ctx.actor_of(props, "child_a").await.unwrap();

        // create second child actor
        let props = Props::new(Child::new);
        let _ = ctx.actor_of(props, "child_b").await.unwrap();
    }

    async fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, _sender: Sender) {
        // up and down: ../select-actor/child_a
        let sel = ctx.select("../select-actor/child_a").unwrap();
        sel.try_tell(msg.clone(), None);

        // child: child_a
        let sel = ctx.select("child_a").unwrap();
        sel.try_tell(msg.clone(), None);

        // absolute: /user/select-actor/child_a
        let sel = ctx.select("/user/select-actor/child_a").unwrap();
        sel.try_tell(msg.clone(), None);

        // // absolute all: /user/select-actor/*
        let sel = ctx.select("/user/select-actor/*").unwrap();
        sel.try_tell(msg.clone(), None);

        // all: *
        let sel = ctx.select("*").unwrap();
        sel.try_tell(msg, None);
    }
}

#[test]
fn select_from_context() {
    let sys = block_on(ActorSystem::new()).unwrap();

    let props = Props::new(SelectTest2::new);
    let actor = block_on(sys.actor_of(props, "select-actor")).unwrap();

    let (probe, listen) = probe();
    block_on(actor.tell(TestProbe(probe), None));

    // seven events back expected:
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
    p_assert_eq!(listen, ());
}

#[test]
fn select_paths() {
    let sys = block_on(ActorSystem::new()).unwrap();

    assert!(sys.select("foo/").is_ok());
    assert!(sys.select("/foo/").is_ok());
    assert!(sys.select("/foo").is_ok());
    assert!(sys.select("/foo/..").is_ok());
    assert!(sys.select("../foo/").is_ok());
    assert!(sys.select("/foo/*").is_ok());
    assert!(sys.select("*").is_ok());

    assert!(sys.select("foo/`").is_err());
    assert!(sys.select("foo/@").is_err());
    assert!(sys.select("!").is_err());
    assert!(sys.select("foo/$").is_err());
    assert!(sys.select("&").is_err());
}

// // *** Dead letters test ***
// struct DeadLettersActor {
//     probe: Option<TestProbe>,
// }

// impl DeadLettersActor {
//     fn new() -> BoxActor<TestMsg> {
//         let actor = DeadLettersActor {
//             probe: None
//         };

//         Box::new(actor)
//     }
// }

// impl Actor for DeadLettersActor {
//     type Msg = TestMsg;

//     fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
//         // subscribe to dead_letters
//         let msg = ChannelMsg::Subscribe(All.into(), ctx.myself());
//         ctx.system.dead_letters().tell(msg, None);
//     }

//     fn receive(&mut self, _: &Context<Self::Msg>, msg: Self::Msg, _: Option<ActorRef<Self::Msg>>) {
//         msg.0.event(()); // notify listen then probe has been received.
//         self.probe = Some(msg.0);
//     }

//     fn other_receive(&mut self, _: &Context<Self::Msg>, msg: ActorMsg<Self::Msg>, _: Option<ActorRef<Self::Msg>>) {
//         if let ActorMsg::DeadLetter(dl) = msg {
//             println!("DeadLetter: {} => {} ({:?})", dl.sender, dl.recipient, dl.msg);
//             self.probe.event(());
//         }
//     }
// }

// #[test]
// fn select_no_actors() {
//     let sys = block_on(ActorSystem::new()).unwrap();

//     let props = Props::new(DeadLettersActor::new);
//     let act = sys.actor_of(props, "dl-subscriber").unwrap();

//     let (probe, listen) = probe();
//     act.tell(TestMsg(probe.clone()), None);

//     // wait for the probe to arrive at the dl sub before doing select
//     listen.recv();

//     let sel = sys.select("nothing-here").unwrap();

//     sel.tell(TestMsg(probe), None);

//     p_assert_eq!(listen, ());
// }
