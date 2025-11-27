m4_dnl One periodic consumer input port with OneItem and DropNewest.
sort Sample = struct sample(value: Bool);

RTCHAN_CHANNEL(port, Sample, 2)
RTCHAN_EVENT_DATA_INPUT(port, Sample, 2)

proc Source = sum v:Bool.
  (port_caller_send_ok(sample(v)) + port_caller_send_full(sample(v))) . Source;

init allow({RTCHAN_CHANNEL_VISIBLE(port), port_input_begin,
            port_input_end, port_complete, port_complete_empty},
  comm({RTCHAN_CHANNEL_COMM(port)},
    Source || RTCHAN_CHANNEL_INIT(port) || RTCHAN_EVENT_DATA_INPUT_INIT(port)));
