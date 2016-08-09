// TODO:
// - Initialize counters/gauges to current values on module load
//   using switch_core_session_count(), switch_core_session_ctl() etc
// - Make bindaddr configurable
// - Allow configuring metrics that can be later references the dialplan
// - Add dimensions to metrics (e.g inbound per profile)
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate freeswitchrs;
extern crate prometheus;
extern crate libc;

use std::sync::{Arc, Mutex};
use std::borrow::Cow;

use freeswitchrs::raw as fsr;
use freeswitchrs::mods::*; // This will get replaced with a mods prelude
use freeswitchrs::Status;
use freeswitchrs::raw::log_level::{INFO, WARNING};

use prometheus::{Registry, Counter, Gauge};

lazy_static! {
    static ref REGISTRY: Arc<Mutex<Registry>> = {
        Arc::new(Mutex::new(prometheus::Registry::new("0.0.0.0".to_string(), 6780)))
    };
    static ref METRIC_HEARTBEAT_COUNT: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_heartbeat_count".to_string(),
                                                     "FreeSWITCH heartbeat count".to_string())))
    };
    static ref METRIC_SESSION_COUNT: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_session_count".to_string(),
                                                     "FreeSWITCH Session Count".to_string())))
    };
    static ref METRIC_SESSIONS_INBOUND: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound".to_string(),
                                                   "FreeSWITCH Inbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_INBOUND_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_answered".to_string(),
                                                   "FreeSWITCH Answered Inbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_OUTBOUND: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound".to_string(),
                                                   "FreeSWITCH Outbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_OUTBOUND_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_answered".to_string(),
                                                   "FreeSWITCH Answered Outbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_answered".to_string(),
                                                   "FreeSWITCH Answered Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_ACTIVE: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_active".to_string(),
                                                   "FreeSWITCH Active Sessions".to_string())))
    };
    static ref METRIC_SESSIONS_ASR: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_asr".to_string(),
                                                   "FreeSWITCH Answer Seizure Ratio".to_string())))
    };
}

fn prometheus_load(mod_int: &ModInterface) -> Status {
    mod_int.add_raw_api("counter_increase", "Increase counter", "counter_increase", counter_increase_api);

    let ref reg = *REGISTRY;

    // At some point we'll have to configure things ...
    //let xml = fsr::xml_open_cfg();
    Registry::start(&reg);
    {
        let mut r = reg.lock().unwrap();
        r.register_counter((*METRIC_HEARTBEAT_COUNT).clone());
        r.register_counter((*METRIC_SESSION_COUNT).clone());
        r.register_counter((*METRIC_SESSIONS_INBOUND).clone());
        r.register_counter((*METRIC_SESSIONS_INBOUND_ANSWERED).clone());
        r.register_counter((*METRIC_SESSIONS_OUTBOUND).clone());
        r.register_counter((*METRIC_SESSIONS_OUTBOUND_ANSWERED).clone());
        r.register_counter((*METRIC_SESSIONS_ANSWERED).clone());
        r.register_gauge((*METRIC_SESSIONS_ACTIVE).clone());
        r.register_gauge((*METRIC_SESSIONS_ASR).clone());
    }

    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::HEARTBEAT, None, |_| {
        METRIC_HEARTBEAT_COUNT.lock().unwrap().increment();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_CREATE, None, |e| {
        METRIC_SESSION_COUNT.lock().unwrap().increment();
        METRIC_SESSIONS_ACTIVE.lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                METRIC_SESSIONS_INBOUND.lock().unwrap().increment();
            } else {
                let outbound = METRIC_SESSIONS_OUTBOUND.lock().unwrap().increment();
                let asr = METRIC_SESSIONS_OUTBOUND_ANSWERED.lock().unwrap().value() / outbound;
                METRIC_SESSIONS_ASR.lock().unwrap().set(asr);
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            unsafe { fslog!(WARNING, "Received channel create event with no call direction: {:?}\n", b); }
        }
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_DESTROY, None, |_| {
        METRIC_SESSIONS_ACTIVE.lock().unwrap().decrement();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_ANSWER, None, |e| {
        METRIC_SESSIONS_ANSWERED.lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                METRIC_SESSIONS_INBOUND_ANSWERED.lock().unwrap().increment();
            } else {
                let answered = METRIC_SESSIONS_OUTBOUND_ANSWERED.lock().unwrap().increment();
                let asr = answered / METRIC_SESSIONS_OUTBOUND.lock().unwrap().value();
                METRIC_SESSIONS_ASR.lock().unwrap().set(asr);
            }
        }
    });
    unsafe {
        fslog!(INFO, "Loaded Prometheus Metrics Module{}", "");
    }
    Ok(())
}

fn prometheus_runtime() -> Status {
    //let ref reg = *REGISTRY;
    //loop {
    //}
    Err(fsr::status::TERM)
}

fn prometheus_unload() -> Status {
    let ref reg = *REGISTRY;
    Registry::stop(&reg);
    Ok(())
}

static MOD_PROMETHEUS_DEF: ModDefinition = ModDefinition {
    name: "mod_prometheus",
    load: prometheus_load,
    runtime: Some(prometheus_runtime),
    shutdown: Some(prometheus_unload)
};

freeswitch_export_mod!(libmod_prometheus_module_interface, MOD_PROMETHEUS_DEF);

#[allow(unused_variables)]
unsafe extern "C" fn counter_increase_api(cmd: *const std::os::raw::c_char,
                                          session: *mut fsr::core_session,
                                          stream: *mut fsr::stream_handle)
                                          -> fsr::status {
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr("OK"));
    fsr::status::SUCCESS
}
