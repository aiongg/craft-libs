//! Linked files: status (embedded, missing, modified, unreadable, refused), the identity check,
//! candidates and ranking, folder remaps and folder matches (`Link`, `LinkIdentity`, `LinkStatus`,
//! `LinkFiles`, `SystemLinkFiles`, `check_candidate`, `Remap`). Every stored path goes through
//! `resolve_absolute` first. File access goes only through `LinkFiles`.

mod search;
