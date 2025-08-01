//! Most likely temporary location of helper functions regarding the parsing of complete server
//! responses.

use thiserror::Error;

use crate::{
    model::{
        comment::{
            level::{CommentUser, LevelComment},
            profile::ProfileComment,
        },
        creator::Creator,
        level::{Level, ListedLevel},
        song::NewgroundsSong,
        user::{profile::Profile, searched::SearchedUser},
    },
    serde::GJFormat,
    DeError,
};

// Since NoneError is not stabilized, we cannot do `impl From<NoneError> for ResponseError<'_>`, so
// this is the next best thing
macro_rules! section {
    ($iter:expr) => {
        match $iter.next() {
            Some(section) => section,
            None => return Err(ResponseError::UnexpectedFormat),
        }
    };
}

#[derive(Debug, Error)]
pub enum ResponseError<'a> {
    /// A deserializer error occured while processing some object contained in the response
    #[error("{0}")]
    De(DeError<'a>), // cannot use #[from] here due to non-'static lifetime

    /// The response was of the form `"-1"`, which is RobTop's version of `HTTP 404 NOT FOUND`
    #[error("not found")]
    NotFound,

    /// The response was not worked in the expected way (too few sections, etc.)
    #[error("unexpected format")]
    UnexpectedFormat,

    #[error("you have been IP banned by Cloudflare")]
    IpBanned,
}

impl<'a> From<DeError<'a>> for ResponseError<'a> {
    fn from(err: DeError<'a>) -> Self {
        ResponseError::De(err)
    }
}

pub fn parse_get_gj_levels_response(response: &str) -> Result<Vec<ListedLevel>, ResponseError> {
    check_response_errors(response)?;

    let mut sections = response.split('#');

    let levels = section!(sections);
    let creators = section!(sections)
        .split('|')
        .filter(|s| !s.is_empty()) // It can happen that segments are completely empty. In this case, split returns an iterator that yields `Some("")`, which would cause an error since the empty string is not parsable
        .map(Creator::from_gj_str)
        .collect::<Result<Vec<Creator>, _>>()?;
    let songs = section!(sections)
        .split("~:~")
        .filter(|s| !s.is_empty())
        .map(NewgroundsSong::from_gj_str)
        .collect::<Result<Vec<NewgroundsSong>, _>>()?;

    levels
        .split('|')
        .map(|fragment| {
            let level: Level<()> = Level::from_gj_str(fragment)?;
            // Note: Cloning is cheap because none of the Thunks is evaluated, so we only have references lying
            // around.
            let creator = creators.iter().find(|creator| creator.user_id == level.creator).cloned();
            let song = level
                .custom_song
                .and_then(|song_id| songs.iter().find(|song| song.song_id == song_id))
                .cloned();

            Ok(Level {
                level_id: level.level_id,
                name: level.name,
                description: level.description,
                version: level.version,
                creator,
                difficulty: level.difficulty,
                downloads: level.downloads,
                main_song: level.main_song,
                gd_version: level.gd_version,
                likes: level.likes,
                length: level.length,
                stars: level.stars,
                featured: level.featured,
                copy_of: level.copy_of,
                two_player: level.two_player,
                custom_song: song,
                coin_amount: level.coin_amount,
                coins_verified: level.coins_verified,
                stars_requested: level.stars_requested,
                is_epic: level.is_epic,
                object_amount: level.object_amount,
                index_46: level.index_46,
                index_47: level.index_47,
                level_data: level.level_data,
            })
        })
        .collect::<Result<_, _>>()
}

pub fn parse_download_gj_level_response(response: &str) -> Result<Level, ResponseError> {
    check_response_errors(response)?;

    let mut sections = response.split('#');

    Ok(Level::from_gj_str(section!(sections))?)
}

pub fn parse_get_gj_user_info_response(response: &str) -> Result<Profile, ResponseError> {
    check_response_errors(response)?;

    Ok(Profile::from_gj_str(response)?)
}

pub fn parse_get_gj_users_response(response: &str) -> Result<SearchedUser, ResponseError> {
    check_response_errors(response)?;

    let mut sections = response.split('#');

    // In the past this used to be a paginating endpoint which performed an infix search on the user
    // name. Now, it performs a full match, and since account names are unique, this endpoint returns at
    // most one object anymore.
    Ok(SearchedUser::from_gj_str(section!(sections))?)
}

pub fn parse_get_gj_comments_response(response: &str) -> Result<Vec<LevelComment>, ResponseError> {
    check_response_errors(response)?;

    let mut sections = response.split('#');

    // The format here is very weird. We have a '|' separated list of (comment, user) pairs, and said
    // pair is separated by a ':'

    section!(sections)
        .split('|')
        .map(|fragment| {
            let mut parts = fragment.split(':');

            if let (Some(raw_comment), Some(raw_user)) = (parts.next(), parts.next()) {
                let mut comment = LevelComment::from_gj_str(raw_comment)?;

                comment.user = if raw_user == "1~~9~~10~~11~~14~~15~~16~" {
                    None
                } else {
                    Some(CommentUser::from_gj_str(raw_user)?)
                };

                Ok(comment)
            } else {
                Err(ResponseError::UnexpectedFormat)
            }
        })
        .collect()
}

pub fn parse_get_gj_acccount_comments_response(response: &str) -> Result<Vec<ProfileComment>, ResponseError> {
    check_response_errors(response)?;

    let mut sections = response.split('#');

    section!(sections)
        .split('|')
        .map(|fragment| Ok(ProfileComment::from_gj_str(fragment)?))
        .collect()
}

fn check_response_errors(response: &str) -> Result<(), ResponseError> {
    if response == "-1" {
        return Err(ResponseError::NotFound);
    }

    if response == "error code: 1005" {
        return Err(ResponseError::IpBanned);
    }

    Ok(())
}
