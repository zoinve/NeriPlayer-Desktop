use base64::{engine::general_purpose::STANDARD, Engine};
use block2::RcBlock;
use objc2::{
    rc::{autoreleasepool, Retained},
    runtime::AnyObject,
    AnyThread,
};
use objc2_app_kit::NSImage;
use objc2_foundation::{NSData, NSMutableDictionary, NSNumber, NSString, NSURL};
use objc2_media_player::{
    MPMediaItemArtwork, MPMediaItemPropertyAlbumTitle, MPMediaItemPropertyArtist,
    MPMediaItemPropertyArtwork, MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle,
    MPNowPlayingInfoCenter,
};
use std::{ptr::NonNull, time::Duration};

pub(super) fn set_metadata(
    title: &str,
    artist: &str,
    album: &str,
    cover_uri: Option<&str>,
    duration: Option<Duration>,
) {
    autoreleasepool(|_| unsafe {
        let info = NSMutableDictionary::<NSString, AnyObject>::new();
        info.insert(MPMediaItemPropertyTitle, &NSString::from_str(title));
        info.insert(MPMediaItemPropertyArtist, &NSString::from_str(artist));
        info.insert(MPMediaItemPropertyAlbumTitle, &NSString::from_str(album));
        if let Some(duration) = duration {
            info.insert(
                MPMediaItemPropertyPlaybackDuration,
                &NSNumber::new_f64(duration.as_secs_f64()),
            );
        }
        if let Some(uri) = cover_uri {
            if let Some(artwork) = load_artwork(uri) {
                info.insert(MPMediaItemPropertyArtwork, &artwork);
            } else {
                log::debug!(target: "media-session", "Could not decode macOS artwork");
            }
        }
        // The media-session worker also publishes progress. Sending the complete dictionary
        // here avoids a second thread racing its read/modify/write of nowPlayingInfo.
        MPNowPlayingInfoCenter::defaultCenter().setNowPlayingInfo(Some(&info));
    });
}

fn load_artwork(uri: &str) -> Option<Retained<MPMediaItemArtwork>> {
    let image = if let Some(data) = uri.strip_prefix("data:image/") {
        let (format, encoded) = data.split_once(',')?;
        if !format.ends_with(";base64") || encoded.len() > (8 * 1024 * 1024_usize).div_ceil(3) * 4 {
            return None;
        }
        NSImage::initWithData(
            NSImage::alloc(),
            &NSData::with_bytes(&STANDARD.decode(encoded).ok()?),
        )?
    } else {
        // Do not fetch from AppKit; remote covers are resolved by the app's cover cache.
        let path = url::Url::parse(uri).ok()?.to_file_path().ok()?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        NSImage::initWithContentsOfURL(NSImage::alloc(), &url)?
    };
    let size = image.size();
    if !(size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0)
    {
        return None;
    }
    // MediaPlayer copies the block, which retains the image for future system requests.
    let handler = RcBlock::new(move |_| NonNull::from(&*image));
    Some(unsafe {
        MPMediaItemArtwork::initWithBoundsSize_requestHandler(
            MPMediaItemArtwork::alloc(),
            size,
            &handler,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_url_artwork_keeps_image_available_to_media_player() {
        let cover = concat!(
            "data:image/png;base64,",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
        );
        autoreleasepool(|_| {
            let artwork = load_artwork(cover).expect("valid PNG artwork");
            let image = unsafe { artwork.imageWithSize(artwork.bounds().size) }
                .expect("MediaPlayer can request the retained image");
            assert_eq!(image.size().width, 1.0);
            assert_eq!(image.size().height, 1.0);
        });
    }

    #[test]
    fn invalid_or_unresolved_artwork_is_omitted() {
        autoreleasepool(|_| {
            assert!(load_artwork("data:image/png;base64,invalid!").is_none());
            assert!(load_artwork("https://example.com/cover.png").is_none());
        });
    }
}
