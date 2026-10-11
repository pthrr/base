m4_dnl OneItem AADL event data input port over the reusable SPSC channel.
m4_dnl $3 is the AADL queue size; $4 is the physical channel capacity.

m4_define(`RTCHAN_EVENT_DATA_INPUT', `
RTCHAN_CHANNEL($1, $2, $4)
act $1_input_begin, $1_input_end;
    $1_complete: $2;
    $1_complete_empty;

proc $1_EventDataInput =
    $1_input_begin .
    (sum queued:Nat. $1_caller_len(queued) .
      ((queued == 0)
        -> $1_input_end . $1_complete_empty . $1_EventDataInput
        <> $1_EventDataTrim(if(queued > $3, Int2Nat(queued - $3), 0))));

proc $1_EventDataTrim(excess: Nat) =
    (excess > 0)
      -> (sum message:$2. $1_caller_recv_ok(message) . $1_EventDataTrim(Int2Nat(excess - 1)))
      <> $1_EventDataFreeze;

proc $1_EventDataFreeze =
    sum message:$2. $1_caller_recv_ok(message) . $1_input_end . $1_complete(message) . $1_EventDataInput;
')

m4_define(`RTCHAN_EVENT_DATA_INPUT_ADAPTER_INIT', `$1_EventDataInput')
m4_define(`RTCHAN_EVENT_DATA_INPUT_INIT', `RTCHAN_CHANNEL_INIT($1) || $1_EventDataInput')
m4_define(`RTCHAN_EVENT_DATA_INPUT_COMM', `RTCHAN_CHANNEL_COMM($1)')
m4_define(`RTCHAN_EVENT_DATA_INPUT_VISIBLE', `RTCHAN_CHANNEL_VISIBLE($1)')
