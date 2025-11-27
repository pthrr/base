m4_dnl An AADL event input uses the event data adapter with a singleton sort.
m4_dnl $2 is the AADL queue size; channel capacity is chosen separately.
m4_define(`RTCHAN_EVENT_INPUT', `
sort $1_Occurrence = struct $1_tick;
RTCHAN_EVENT_DATA_INPUT($1, $1_Occurrence, $2)
')

m4_define(`RTCHAN_EVENT_INPUT_INIT', `RTCHAN_EVENT_DATA_INPUT_INIT($1)')
