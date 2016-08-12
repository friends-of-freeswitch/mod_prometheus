FreeSWITCH Prometheus Module
--------------------------------

This module exposes FreeSWITCH metrics for scraping by
`Prometheus
<https://prometheus.io/>`_.

You'll need to have FreeSWITCH rust bindings
`freeswitchrs
<https://gitlab.com/wiresight/freeswitchrs/>`_.

Include this module in the freeswitchrs directory to build it.

You can also use FreeSWITCH ESL APIs to increase custom counters or gauges:::.

    fscli> prom_counter_increment my_counter

    fscli> prom_counter_increment my_counter 100

    fscli> prom_gauge_set my_gauge 500

    fscli> prom_gauge_increment my_gauge
    fscli> prom_gauge_decrement my_gauge 2

As All FreeSWITCH APIs, these functions can be used from the XML dialplan or the command line.
