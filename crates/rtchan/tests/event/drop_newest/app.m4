m4_dnl An AADL event port: every occurrence has the same singleton payload.
m4_dnl Queue size and ring capacity are two, so overflow is DropNewest.
RTCHAN_EVENT_INPUT(port, 2, 2)

proc Source = (port_caller_send_ok(port_tick) + port_caller_send_full(port_tick)) . Source;

init allow({RTCHAN_EVENT_INPUT_VISIBLE(port), port_input_begin,
            port_input_end, port_complete, port_complete_empty},
  comm({RTCHAN_EVENT_INPUT_COMM(port)},
    Source || RTCHAN_EVENT_INPUT_INIT(port)));
