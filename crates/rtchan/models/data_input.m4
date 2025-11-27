m4_dnl A sampled AADL data input is an event data port with queue size one
m4_dnl and a retained value. Compose with RTCHAN_CHANNEL of capacity K.
m4_define(`RTCHAN_DATA_INPUT', `
RTCHAN_EVENT_DATA_INPUT($1, $2, 1)
sort $1_Latest = struct $1_none | $1_some($1_value: $2);

act $1_keep, $1_kept: $2;
    $1_keep_empty, $1_kept_empty;
    $1_frozen: $1_Latest # Bool;

proc $1_DataInput(latest: $1_Latest) =
    (sum message:$2. ($1_keep(message) | $1_frozen($1_some(message), true)) . $1_DataInput($1_some(message)))
  + ($1_keep_empty | $1_frozen(latest, false)) . $1_DataInput(latest);
')

m4_define(`RTCHAN_DATA_INPUT_VISIBLE', `RTCHAN_CHANNEL_VISIBLE($1),
    $1_kept|$1_frozen, $1_kept_empty|$1_frozen')

m4_define(`RTCHAN_DATA_INPUT_INIT', `RTCHAN_EVENT_DATA_INPUT_INIT($1) || $1_DataInput($1_none)')
