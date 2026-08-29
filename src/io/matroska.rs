use matroska_demuxer::{TrackEntry, TrackType};

const HEVC_CODEC_ID: &str = "V_MPEGH/ISO/HEVC";

pub(crate) fn is_hevc_video_track(track: &TrackEntry) -> bool {
    track.track_type() == TrackType::Video && track.codec_id() == HEVC_CODEC_ID
}
