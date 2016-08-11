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

    fscli> prom_counter_increase my_counter

    fscli> prom_counter_increase my_counter 100

    fscli> prom_gauge_set my_gauge 500

As All FreeSWITCH APIs, these functions can be used from the XML dialplan or the command line.
