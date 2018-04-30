FreeSWITCH Prometheus Module
----------------------------

**WARNING**: This module has serious bugs and does not produce reliable metrics at the moment. While I hope I can find the time to fix it soon, I'd rather people not waste their
time trying to install and use it when the results won't be usable in a production environment.

This module exposes FreeSWITCH metrics for scraping by
`Prometheus
<https://prometheus.io/>`_.

mod_prometheus is built upon  
`FreeSWITCH Rust bindings
<https://gitlab.com/wiresight/freeswitchrs/>`_.


Install
=======

Installation instructions::

    # Check for and install the Rust compiler
    $ rustc -V || curl -sSf https://static.rust-lang.org/rustup.sh | sh

    # Clone the project and build it:
    $ git clone https://github.com/moises-silva/mod_prometheus.git
    $ cd mod_prometheus
    $ cargo build

    # Copy the module to your FreeSWITCH modules directory:
    $ sudo cp target/debug/libmod_prometheus.so `fs_cli -x 'global_getvar mod_dir'`/mod_prometheus.so

    # Load the module:
    $ fs_cli -x 'load mod_prometheus'

    # Make sure it's loaded and listening to TCP port 9282
    $ fs_cli -x 'module_exists mod_prometheus'
    true

    $ netstat -nl | grep 9282
    tcp        0      0 0.0.0.0:9282            0.0.0.0:*               LISTEN
    
    # For auto-load the module add this line at the end of your modules.conf 
    $ sudo vi /etc/freeswitch/autoload_configs/modules.conf.xml

        <load module="mod_prometheus"/>
    
Now you can access your host at port 9282 to check your metrics::

    $ curl http://127.0.0.1:9282/metrics

The /metrics url path is not required but it could be required in the future as it's recommended by the Prometheus guidelines.

Metrics
=======

These are the metrics provided by default::

Counters::

    freeswitch_heartbeats_total
    freeswitch_registration_attempts_total
    freeswitch_registration_failures_total
    freeswitch_registrations_total
    freeswitch_sessions_total
    freeswitch_sessions_answered_total
    freeswitch_sessions_failed_total
    freeswitch_sessions_inbound_total
    freeswitch_sessions_inbound_answered_total
    freeswitch_sessions_inbound_failed_total
    freeswitch_sessions_outbound_total
    freeswitch_sessions_outbound_answered_total
    freeswitch_sessions_outbound_failed_total

Gauges::

    freeswitch_sessions_active
    freeswitch_sessions_asr
    freeswitch_registrations_active

You can also use FreeSWITCH ESL APIs to create your own counters or gauges::

    fscli> prom_counter_increment my_counter

    fscli> prom_counter_increment my_counter 100

    fscli> prom_gauge_set my_gauge 500

    fscli> prom_gauge_increment my_gauge
    fscli> prom_gauge_decrement my_gauge 2

As all FreeSWITCH APIs, these functions can be used from the XML dialplan or the command line.
