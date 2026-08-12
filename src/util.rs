use {td_api::{FormattedText, InputChecklist, InputChecklistTask, InputFile, InputMessageContent, InputPollOption, InputPollType, MessageContent, PollType}};

pub macro info($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::cprintln!(
            "<cyan>{}</cyan>",
            format!($($arg)*)
        );
    } else {
        println!($($arg)*);
    }
}

pub macro error($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::ceprintln!(
            "<red>{}</red>",
            format!($($arg)*)
        );
    } else {
        eprintln!($($arg)*);
    }
}

pub macro success($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::cprintln!(
            "<green>{}</green>",
            format!($($arg)*)
        );
    } else {
        println!($($arg)*);
    }
}

pub fn shorten(max_len_before_shortening: i32, s: &str) -> String {
    if max_len_before_shortening < 0 {
        return s.to_string();
    }
    let max_len = max_len_before_shortening as usize;
    if s.chars().count() > max_len {
        s.chars().take(max_len).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}

pub fn message_content_to_input_message_content(content: MessageContent) -> Option<InputMessageContent> {
    match content {
        MessageContent::MessageText {
            text,
            link_preview_options,
            ..
        } => Some(InputMessageContent::InputMessageText {
            text,
            link_preview_options,
            clear_draft: false,
        }),
 
        MessageContent::MessageAnimation {
            animation,
            caption,
            show_caption_above_media,
            has_spoiler,
			..
        } => Some(InputMessageContent::InputMessageAnimation {
            animation: InputFile::Id {
                id: animation.animation.id,
            },
            thumbnail: None,
            added_sticker_file_ids: Vec::new(),
            duration: animation.duration,
            width: animation.width,
            height: animation.height,
            caption: Some(caption),
            show_caption_above_media,
            has_spoiler,
        }),
 
        MessageContent::MessageAudio { audio, caption } => {
            Some(InputMessageContent::InputMessageAudio {
                audio: InputFile::Id { id: audio.audio.id },
                album_cover_thumbnail: None,
                duration: audio.duration,
                title: audio.title,
                performer: audio.performer,
                caption: Some(caption),
            })
        }
 
        MessageContent::MessageDocument { document, caption } => {
            Some(InputMessageContent::InputMessageDocument {
                document: InputFile::Id {
                    id: document.document.id,
                },
                thumbnail: None,
                disable_content_type_detection: false,
                caption: Some(caption),
            })
        }
 
        MessageContent::MessagePaidMedia {
            star_count,
            caption,
            show_caption_above_media,
			..
        } => Some(InputMessageContent::InputMessagePaidMedia {
            star_count,
            // NOTE: `PaidMedia` (received) -> `InputPaidMedia` (to send) isn't a pure
            // field-rename (received paid media wraps a MessageContent variant per item,
            // e.g. Photo/Video, while InputPaidMedia wraps the corresponding InputFile).
            // Fill this in against your InputPaidMedia definition; left empty here since
            // it's not one of the types you provided.
            paid_media: Vec::new(),
            caption: Some(caption),
            show_caption_above_media,
            payload: String::new(),
        }),
 
        MessageContent::MessagePhoto {
            photo,
            caption,
            show_caption_above_media,
            has_spoiler,
            ..
        } => Some(InputMessageContent::InputMessagePhoto {
            photo: InputFile::Id {
                // Largest available size is last in `sizes`.
                id: photo.sizes.last()?.photo.id,
            },
            thumbnail: None,
            video: None,
            added_sticker_file_ids: Vec::new(),
            width: photo.sizes.last().map(|s| s.width).unwrap_or_default(),
            height: photo.sizes.last().map(|s| s.height).unwrap_or_default(),
            caption: Some(caption),
            show_caption_above_media,
            self_destruct_type: None,
            has_spoiler,
        }),
 
        MessageContent::MessageSticker { sticker, .. } => {
            Some(InputMessageContent::InputMessageSticker {
                sticker: InputFile::Id {
                    id: sticker.sticker.id,
                },
                thumbnail: None,
                width: sticker.width,
                height: sticker.height,
                emoji: sticker.emoji,
            })
        }
 
        MessageContent::MessageVideo {
            video,
            caption,
            show_caption_above_media,
            has_spoiler,
            ..
        } => Some(InputMessageContent::InputMessageVideo {
            video: InputFile::Id { id: video.video.id },
            thumbnail: None,
            cover: None,
            start_timestamp: 0,
            added_sticker_file_ids: Vec::new(),
            duration: video.duration,
            width: video.width,
            height: video.height,
            supports_streaming: video.supports_streaming,
            caption: Some(caption),
            show_caption_above_media,
            self_destruct_type: None,
            has_spoiler,
        }),
 
        MessageContent::MessageVideoNote { video_note, .. } => {
            Some(InputMessageContent::InputMessageVideoNote {
                video_note: InputFile::Id {
                    id: video_note.video.id,
                },
                thumbnail: None,
                duration: video_note.duration,
                length: video_note.length,
                self_destruct_type: None,
            })
        }
 
        MessageContent::MessageVoiceNote {
            voice_note,
            caption,
            ..
        } => Some(InputMessageContent::InputMessageVoiceNote {
            voice_note: InputFile::Id {
                id: voice_note.voice.id,
            },
            duration: voice_note.duration,
            waveform: voice_note.waveform,
            caption: Some(caption),
            self_destruct_type: None,
        }),
 
        MessageContent::MessageLocation {
            location,
            live_period,
            heading,
            proximity_alert_radius,
            ..
        } => Some(InputMessageContent::InputMessageLocation {
            location,
            live_period,
            heading,
            proximity_alert_radius,
        }),
 
        MessageContent::MessageVenue { venue } => {
            Some(InputMessageContent::InputMessageVenue { venue })
        }
 
        MessageContent::MessageContact { contact } => {
            Some(InputMessageContent::InputMessageContact { contact })
        }
 
        MessageContent::MessageDice { emoji, .. } => {
            Some(InputMessageContent::InputMessageDice {
                emoji,
                clear_draft: false,
            })
        }
 
        MessageContent::MessageGame { game } => Some(InputMessageContent::InputMessageGame {
            bot_user_id: 0,
            game_short_name: game.short_name,
        }),
 
        MessageContent::MessageInvoice {
            ..
        } => None,

        MessageContent::MessagePoll { poll, .. } => Some(InputMessageContent::InputMessagePoll {
            question: poll.question,
            options: poll
                .options
                .into_iter()
                .map(|o| InputPollOption {
                    text: o.text,
                    media: None,
                })
                .collect(),
            description: FormattedText {
                text: String::new(),
                entities: Vec::new(),
            },
            media: None,
            is_anonymous: poll.is_anonymous,
            allows_multiple_answers: poll.allows_multiple_answers,
            allows_revoting: poll.allows_revoting,
            members_only: poll.members_only,
            country_codes: poll.country_codes,
            shuffle_options: false,
            hide_results_until_closes: false,
            r#type: Box::new(if let PollType::Regular = poll.r#type {
				InputPollType::Regular { allow_adding_options: true }
			} else if let PollType::Quiz { correct_option_ids, explanation, explanation_media } = poll.r#type {
				InputPollType::Quiz { correct_option_ids, explanation, explanation_media: if let Some(media) = explanation_media {
					let media = *media;
					message_content_to_input_message_content(media)
				} else {
					None
				}}
			} else {
				unreachable!()
			}),
            open_period: poll.open_period,
            close_date: poll.close_date,
            is_closed: poll.is_closed,
        }),
 
        MessageContent::MessageChecklist { list: checklist } => {
            Some(InputMessageContent::InputMessageChecklist {
                checklist: InputChecklist {
                    title: checklist.title,
                    tasks: checklist
                        .tasks
                        .into_iter()
                        .map(|t| InputChecklistTask {
                            id: t.id,
                            text: t.text,
                        })
                        .collect(),
                    others_can_add_tasks: checklist.others_can_add_tasks,
                    others_can_mark_tasks_as_done: checklist.others_can_mark_tasks_as_done,
                },
            })
        }
 
        MessageContent::MessageStory {
            story_poster_chat_id,
            story_id,
            ..
        } => Some(InputMessageContent::InputMessageStory {
            story_poster_chat_id,
            story_id,
        }),
 
        // Every other MessageContent variant (chat events, service messages, payment
        // receipts, poll/gift/premium notifications, etc.) has no sendable
        // InputMessageContent equivalent.
        _ => None,
    }
}
