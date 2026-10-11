m4_dnl Sampled AADL data input: latest-value selection and retention.
m4_dnl $3 is the physical channel capacity K.
m4_define(`RTCHAN_DATA_INPUT', `
RTCHAN_CHANNEL($1, $2, $3)
sort $1_Latest = struct $1_none | $1_some($1_value: $2);

act $1_input_begin, $1_input_end;
    $1_frozen: $1_Latest # Bool;

proc $1_DataInput(latest: $1_Latest) =
    $1_input_begin .
    (sum queued:Nat. $1_caller_len(queued) .
      ((queued == 0)
        -> $1_input_end . $1_frozen(latest, false) . $1_DataInput(latest)
        <> $1_DataTrim(if(queued > 1, Int2Nat(queued - 1), 0), latest)));

proc $1_DataTrim(excess: Nat, latest: $1_Latest) =
    (excess > 0)
      -> (sum message:$2. $1_caller_recv_ok(message) .
          $1_DataTrim(Int2Nat(excess - 1), latest))
      <> $1_DataFreeze(latest);

proc $1_DataFreeze(latest: $1_Latest) =
    (sum message:$2. $1_caller_recv_ok(message) . $1_input_end .
      $1_frozen($1_some(message), true) . $1_DataInput($1_some(message)));
')

m4_define(`RTCHAN_DATA_INPUT_VISIBLE', `RTCHAN_CHANNEL_VISIBLE($1), $1_frozen')

m4_define(`RTCHAN_DATA_INPUT_INIT', `RTCHAN_CHANNEL_INIT($1) || $1_DataInput($1_none)')
m4_define(`RTCHAN_DATA_INPUT_COMM', `RTCHAN_CHANNEL_COMM($1)')
