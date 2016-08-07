#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate freeswitchrs;
extern crate prometheus;
extern crate libc;

use std::sync::{Arc, Mutex};
//use std::borrow::Cow;

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
    static ref METRIC_SESSIONS_ACTIVE: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_active".to_string(),
                                                   "FreeSWITCH Active Sessions".to_string())))
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
        r.register_gauge((*METRIC_SESSIONS_ACTIVE).clone());
    }

    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::HEARTBEAT, None, |e| {
        unsafe {
            if let Some(sesscount) = e.header("Session-Count") {
                fslog!(WARNING, "Received heartbeat with Session-Count: {}", sesscount);
                METRIC_HEARTBEAT_COUNT.lock().unwrap().increment();
            } else {
                fslog!(WARNING, "Received heartbeat without Session-Count header {}", "");
            }
        }
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_CREATE, None, |_| {
        unsafe {
            fslog!(WARNING, "Received channel create{}", "");
            METRIC_SESSION_COUNT.lock().unwrap().increment();
            METRIC_SESSIONS_ACTIVE.lock().unwrap().increment();
        }
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_DESTROY, None, |_| {
        unsafe {
            fslog!(WARNING, "Received channel create{}", "");
            METRIC_SESSIONS_ACTIVE.lock().unwrap().decrement();
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
