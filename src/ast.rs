use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub expression: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    BooleanOp {
        operator: BooleanOperator,
        left: Box<Expression>,
        right: Option<Box<Expression>>,
        span: Span,
    },

    Group {
        expression: Box<Expression>,
        span: Span,
    },

    Proximity {
        operator: ProximityOperator,
        terms: Vec<Expression>,
        span: Span,
    },

    Field {
        field: FieldType,
        value: Box<Expression>,
        span: Span,
    },

    Range {
        field: Option<FieldType>,
        start: String,
        end: String,
        span: Span,
    },

    Term {
        term: Term,
        span: Span,
    },

    Comment {
        text: String,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BooleanOperator {
    And,
    Or,
    Not,
}

impl BooleanOperator {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "AND" => Some(Self::And),
            "OR" => Some(Self::Or),
            "NOT" => Some(Self::Not),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::Not => "NOT",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProximityOperator {
    Proximity { distance: Option<u32> },
    Near { distance: u32 },
    NearForward { distance: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Title,
    Site,
    Url,
    Author,
    Links,
    
    Continent,
    Country,
    Region,
    City,
    Latitude,
    Longitude,
    
    Language,
    ChannelId,
    
    AuthorGender,
    AuthorVerified,
    AuthorVerifiedType,
    AuthorFollowers,
    BlogName,
    ParentBlogName,
    RootBlogName,
    ParentPostId,
    RootPostId,
    Tags,
    BrandIds,
    Objects,
    EngagementType,
    EngagingWith,
    EngagingWithGuid,
    Guid,
    ImageType,
    ItemReview,
    Rating,
    MinuteOfDay,
    PubType,
    PublisherSubType,
    Publication,
    RedditAuthorFlair,
    RedditPostFlair,
    RedditSpoiler,
    SensitiveContent,
    Subreddit,
    SubredditNSFW,
    SubredditTopics,
    TopLevelDomain,
    WeblogTitle,
    Sentiment,
}

impl FieldType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "title" => Some(Self::Title),
            "site" => Some(Self::Site),
            "url" => Some(Self::Url),
            "author" => Some(Self::Author),
            "links" => Some(Self::Links),
            "continent" => Some(Self::Continent),
            "country" => Some(Self::Country),
            "region" => Some(Self::Region),
            "city" => Some(Self::City),
            "latitude" => Some(Self::Latitude),
            "longitude" => Some(Self::Longitude),
            "language" => Some(Self::Language),
            "channelid" => Some(Self::ChannelId),
            "authorgender" => Some(Self::AuthorGender),
            "authorverified" => Some(Self::AuthorVerified),
            "authorverifiedtype" => Some(Self::AuthorVerifiedType),
            "authorfollowers" => Some(Self::AuthorFollowers),
            "blogname" => Some(Self::BlogName),
            "parentblogname" => Some(Self::ParentBlogName),
            "rootblogname" => Some(Self::RootBlogName),
            "parentpostid" => Some(Self::ParentPostId),
            "rootpostid" => Some(Self::RootPostId),
            "tags" => Some(Self::Tags),
            "brandids" => Some(Self::BrandIds),
            "objects" => Some(Self::Objects),
            "engagementtype" => Some(Self::EngagementType),
            "engagingwith" => Some(Self::EngagingWith),
            "engagingwithguid" => Some(Self::EngagingWithGuid),
            "guid" => Some(Self::Guid),
            "imagetype" => Some(Self::ImageType),
            "itemreview" => Some(Self::ItemReview),
            "rating" => Some(Self::Rating),
            "minuteofday" => Some(Self::MinuteOfDay),
            "pubtype" => Some(Self::PubType),
            "publishersubtype" => Some(Self::PublisherSubType),
            "publication" => Some(Self::Publication),
            "redditauthorflair" => Some(Self::RedditAuthorFlair),
            "redditpostflair" => Some(Self::RedditPostFlair),
            "redditspoiler" => Some(Self::RedditSpoiler),
            "sensitivecontent" => Some(Self::SensitiveContent),
            "subreddit" => Some(Self::Subreddit),
            "subredditnsfw" => Some(Self::SubredditNSFW),
            "subreddittopics" => Some(Self::SubredditTopics),
            "topleveldomain" => Some(Self::TopLevelDomain),
            "weblogtitle" => Some(Self::WeblogTitle),
            "sentiment" => Some(Self::Sentiment),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Site => "site",
            Self::Url => "url",
            Self::Author => "author",
            Self::Links => "links",
            Self::Continent => "continent",
            Self::Country => "country",
            Self::Region => "region",
            Self::City => "city",
            Self::Latitude => "latitude",
            Self::Longitude => "longitude",
            Self::Language => "language",
            Self::ChannelId => "channelId",
            Self::AuthorGender => "authorGender",
            Self::AuthorVerified => "authorVerified",
            Self::AuthorVerifiedType => "authorVerifiedType",
            Self::AuthorFollowers => "authorFollowers",
            Self::BlogName => "blogName",
            Self::ParentBlogName => "parentBlogName",
            Self::RootBlogName => "rootBlogName",
            Self::ParentPostId => "parentPostId",
            Self::RootPostId => "rootPostId",
            Self::Tags => "tags",
            Self::BrandIds => "brandIds",
            Self::Objects => "objects",
            Self::EngagementType => "engagementType",
            Self::EngagingWith => "engagingWith",
            Self::EngagingWithGuid => "engagingWithGuid",
            Self::Guid => "guid",
            Self::ImageType => "imageType",
            Self::ItemReview => "itemReview",
            Self::Rating => "rating",
            Self::MinuteOfDay => "minuteOfDay",
            Self::PubType => "pubType",
            Self::PublisherSubType => "publisherSubType",
            Self::Publication => "publication",
            Self::RedditAuthorFlair => "redditAuthorFlair",
            Self::RedditPostFlair => "redditPostFlair",
            Self::RedditSpoiler => "redditSpoiler",
            Self::SensitiveContent => "sensitiveContent",
            Self::Subreddit => "subreddit",
            Self::SubredditNSFW => "subredditNSFW",
            Self::SubredditTopics => "subredditTopics",
            Self::TopLevelDomain => "topLevelDomain",
            Self::WeblogTitle => "weblogTitle",
            Self::Sentiment => "sentiment",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Word { value: String },
    Phrase { value: String },
    Wildcard { value: String },
    Replacement { value: String },
    CaseSensitive { value: String },
    Hashtag { value: String },
    Mention { value: String },
    Emoji { value: String },
}