m4_dnl One sporadic consumer input port with OneItem and DropNewest.
m4_dnl Everything below the rtchan macros is application code: the source
m4_dnl wakes the consumer after each accepted send, the latch stands for any
m4_dnl wakeup that remembers one notification until the next wait (a binary
m4_dnl semaphore, an eventfd, Thread::unpark), and the scheduler waits, then
m4_dnl dispatches while input is pending. Minimum separation is timing and is
m4_dnl abstracted away.
sort Item = struct i0 | i1 | i2;

RTCHAN_CHANNEL(port, Item, 2)
RTCHAN_EVENT_DATA_INPUT(port, Item, 2)

act app_caller_notify, app_latch_notify, app_notify,
    app_caller_wait, app_latch_wait, app_wait,
    app_sched_begin, app_sched_end, port_begin, port_end;

proc Latch(set: Bool) =
    app_latch_notify . Latch(true)
  + (set -> app_latch_wait . Latch(false));

m4_dnl if tx.try_send(v).is_ok() { wake() }
m4_define(`SEND_AND_WAKE',
  `(port_caller_send_ok($1) . app_caller_notify + port_caller_send_full($1))')

proc Source = SEND_AND_WAKE(i0) . SEND_AND_WAKE(i1) . SEND_AND_WAKE(i2);

m4_dnl has_pending() is one length read.
proc Scheduler = app_caller_wait . Drain;
proc Drain = sum n:Nat. port_caller_len(n) .
    ((n > 0) -> app_sched_begin . app_sched_end . Drain <> Scheduler);

init allow({RTCHAN_CHANNEL_VISIBLE(port), app_notify, app_wait,
            port_begin, port_end, port_complete, port_complete_empty},
  comm({RTCHAN_CHANNEL_COMM(port),
        app_caller_notify|app_latch_notify -> app_notify,
        app_caller_wait|app_latch_wait -> app_wait,
        port_input_begin|app_sched_begin -> port_begin,
        port_input_end|app_sched_end -> port_end},
    Source || Scheduler || RTCHAN_EVENT_DATA_INPUT_INIT(port) || Latch(false)
      || RTCHAN_CHANNEL_INIT(port)));
