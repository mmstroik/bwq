---
title: "Query Operators"
url: "https://consumer-research-help.brandwatch.com/hc/en-us/articles/360012098098-Query-Operators"
---

## Operators

| **Operator** | **Example** | **Description** |
| Quotes " " | "apple juice" | Will find Mentions of the exact phrase 'apple juice' on any webpage. |
| AND | apple AND juice | Will find mentions of 'apple' and 'juice' on the **same** webpage. Must be capitalized. |
| OR | apple OR juice | Will find mentions of 'apple' or mentions of 'juice' on any webpage. Must be capitalized. |
| NOT | apple NOT juice | Will find mentions of 'apple' on a page as long as 'juice' is not mentioned on that page. Must be capitalized. |
| Brackets ( ) | (apple AND juice) OR (apple AND sauce) | Will find mentions of 'apple' and 'juice' on the same page or mentions of 'apple' and 'sauce' on the same page. |
| Proximity  ~ | "apple juice"~5 | Will find mentions of the exact phrase 'apple juice'  and mentions of 'apple' and 'juice' within 5 words of each other, e.g. 'This drink was made with fresh apple, orange and pear juice'. |
| NEAR/x | ((apple OR orange) NEAR/5 (smartphone OR phone)) | Will find mentions of 'apple' within 5 words of 'smartphone' or 'phone' and mentions of 'orange' within 5 words of 'smartphone' or 'phone'. |
| NEAR/xf | (logitech NEAR/2f keyboard) | Will find mentions where 'logitech' appears before 'keyboard' with 2 or fewer words in-between e.g. 'logitech keyboard', 'Logitech Bluetooth Keyboard', 'Logitech Solar Keyboard' etc. |
| NEAR ~ | ((apple OR orange) AND (smartphone OR phone))~5 | In this instance , NEAR is represented by identifying your two sets of keywords and following them with a tilde and the maximum number of words separating them.  **For advanced users:** _This option does not allow more than one layer of nesting. If you require more advanced nesting, please use the **NEAR/x** operator._ |
| Wildcard \* | complain\* | Will find mentions with the root word complain, e.g. 'complain', 'complaints', 'complained' etc. There is a limit of 150 on short wildcards (2 characters) e.g. \*ab. NB: Only compatible with plain text and can only be used within or at the end of a word, not at the beginning. |
| Replacement ? | customi?e | Will find mentions where ? can be replaced by another letter to accommodate for variations of spellings, such as 'customise' AND 'customize'. |
| title: | title:"apple juice" | Will find any mentions where 'apple juice' appears in the page title. |
| Comments <<< >>> | <<<my comment here\>>> | Allows you to add your own comments into different sections of the Query string. _This does not work within a bracketed string._ |
| Upper Case Sensitive Matching {} | {BT} | Will only retrieve Mentions where upper case letters are used in the exact way specified inside the brackets. Upper case matching works for words with up to 20 characters from 1st July 2021 and up to 5 characters prior to that. |
| Accents Ä, è, ñ | niño | Accented keywords will only match Mentions with words spelled exactly as you type them. However non-accented words will match everything regardless of the use of accented characters. _To prevent specific accented words to match you will need to use the NOT operator. E.g. Nino NOT niño_ |
| continent: | continent:europe AND "apple juice" | Will only find mentions of the exact phrase 'apple juice' that have been identified as from Europe. To find a location code, see [Location Codes](https://consumer-research-help.brandwatch.com/hc/en-us/article_attachments/11421712445725) or use the 'Locations look up' tool available in the "Find location codes" section of the Query Editor. |
| country: | country:gbr AND "apple juice" | Will only find mentions of the exact phrase 'apple juice' that have been identified as from the UK. To find a location code, see [Location Codes](#https://consumer-research-help.brandwatch.com/hc/en-us/article_attachments/11421712445725) or use the 'Locations look up' tool available in the "Find location codes" section of the Query Editor. |
| region: | region:usa.fl AND "apple juice" | Will only find mentions of the exact phrase 'apple juice' that have been identified as from the specified state, region or province (in this example Florida, USA). It consists of a country code followed by a region's name or abbreviation. To find a location code, see [Location Codes](https://consumer-research-help.brandwatch.com/hc/en-us/article_attachments/11421712445725) or use the 'Locations look up' tool available in the "Find location codes" section of the Query Editor. |
| city: | city:"deu.berlin.berlin" AND "apple juice" | Will only find mentions of the exact phrase 'apple juice' that have been identified from Berlin, Germany. It has 3 components and consists of a country code followed by a region code, followed by a city code. _It is required to put city codes within double quotes where a space forms part of the name, such as 'New York'._ To find a location code, see [Location Codes](https://consumer-research-help.brandwatch.com/hc/en-us/article_attachments/11421712445725) or use the 'Locations look up' tool available in the "Find location codes" section of the Query Editor. |
| language: | language:en | Will only find mentions in the specified language. We use the ISO 639-1 codes found [here](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes). |
| latitude: | latitude:\[41 TO 44\]AND longitude:\[-73 TO -69\] | This can be used to indicate specific coordinates you'd like to associate with your Query. It can be used alone or with the longitude: operator. |
| longitude: | longitude:\[-73 TO -69\] AND latitude:\[41 TO 44\] | This can be used to indicate specific coordinates you'd like to associate with your Query. It can be used alone or with the latitude: operator. |
| site: | site:twitter.com AND "apple juice" | Will find mentions on a particular site (in this example, any mention of 'apple juice' on X (Twitter)). _Be sure not to include 'www.'_ |
| url: | url:"msn.com/news" AND "Simon Cowell" | Will find mentions on a particular part of a site (in this example any mention of 'Simon Cowell' on the news section of the MSN website). |
| channelId: | channelId:"141273439246434" AND "summer" | Will find mentions on a specified channel (in this example any mention of 'summer' on the Blue Ridge Parkway Facebook page). |
| author: | author:ladygaga | Will find mentions with a specific author name (in this example any posts by any author called 'ladygaga'). Will NOT work with Tumblr (see "blogName:") and some other forums (see "weblogTitle:"). Note: Consumer Research treats "." as spaces. Facebook author names can include "." , so to use this operator with author names that include".", these should be quoted and the "." replaced with a space, as with any other operator over multiple word terms. For example, author:brandwatch.office should be chanced to author:"brandwatch office" |
| links: | links:msn.com | For X (Twitter) only, will find mentions containing links to the msn.com website. Note: In this case, if the links: operator had not been used, mentions containing shortened links to msn.com would not have been picked up. |

---

## Advanced operators

| Operator | Example | Description |
| authorGender: | authorGender:F | Available for Facebook, Forums, and Reviews data from March 8, 2014 and X (Twitter) data from April 18, 2014 this operator allows you to specify the Gender of the authors you are interested in. Can be set to F = Female or M = Male. |
| authorVerified: | authorVerified:true | Used to search for posts (tweets) by [X (Twitter) verified authors](https://help.twitter.com/en/managing-your-account/about-twitter-verified-accounts). Can be set to true or false. |
| authorVerifiedType: | authorVerifiedType:blue | Used to search for posts (tweets) by specific X (Twitter) verified authors. Can be set to blue, business, or government. _Note: The above terms must be written all lowercase._ |
| authorFollowers: | authorFollowers:\[500 TO 100000\] | Will find posts posted by X (Twitter) authors with a follower count within the specified range. |
| blogName: | blogName:comedycentral | Track original and reblogged mentions from a specific Tumblr page (in this example 'comedycentral'). |
| parentBlogName: | parentBlogName:comedycentral | Track content reblogged from a specific Tumblr page (in this example 'comedycentral'). |
| rootBlogName: | rootBlogName:comedycentral | Track mentions reblogged from a specific page and reblogs of that same content on Tumblr (in this example 'comedycentral'). |
| parentPostId: | parentPostId:129596930020 | Track reblogs of a specific post ID from a specific page on Reddit and Tumblr and reblogs of those (in this example post ID '129596930020'). |
| rootPostId: | rootPostId:129596930020 | Track reblogs of a specific post on Reddit and Tumblr and reblogs of those (in this example post ID '129596930020'). |
| tags: | tags:photography | Track mentions that include the specified tag on Tumblr (in this example those tagged with 'photography'). |
| brandIds: | brandIds:12 AND brandIds:14 | Track mentions that contain a Logo from the specified Logo Brand ID/s (in this example Logos from both Brand ID 12 and 14). The full list of available logo IDs can be found [here](https://consumer-research-help.brandwatch.com/hc/en-us/articles/360012098078). |
| objects: | objects:3902 | Track mentions that contain an object within an image that relates to the object ID 3902. A list of object IDs can be downloaded [here](https://consumer-research-help.brandwatch.com/hc/article_attachments/8593983101213). |
| engagementType: | engagementType:COMMENT engagementType:REPLY engagementType:RETWEET engagementType:QUOTE | Track Facebook and Instagram comments. Can also be used to isolate original posts only in Reddit (not threaded comments). Track X (Twitter) replies/comments. Track reposts (retweets). Track quote posts (quote tweets). _Note: Values are case-sensitive. When the operator is used in a string with a keyword in quotes, e.g. ""john mayer" AND engagementType:QUOTE", the query will return keyword mentions only in the specified engagementType, e.g. the quote post, not the original element. We recommend using this operator in combination with the engagingWithGuid: operator if you would like to reference a specific post ID._ |
| engagingWith: | engagingWith:brandwatch | Track replies or reposts (retweets) of a specific X (Twitter) handle (in this example 'brandwatch'). Also tracks owned Instagram accounts. |
| engagingWithGuid: | engagingWithGuid:   857284431631011841 engagingWithGuid:1015572792854 | Track reposts (retweets) and replies to a specific post (tweet) ID, which can be obtained from the end of a post (tweet) URL, e.g. https://twitter.com/crimsonhexagon/status/857284431631011841 Track replies to a specific Facebook Post ID, which can be taken from a Facebook Post URL in order, e.g.   https://www.facebook.com/posts/1015572792854 _Note: Use parentPostID and rootPostID for Reddit._ |
| guid: | guid:857284431631011841 | Track specific post (tweet) from URL. |
|  | guid:1277374998\_1015572792854 | Track a specific Facebook post. Use Page ID\_Post ID, which are taken from a Facebook Post URL in order - see above. |
| imageType: | imageType:image | Track posts that contain images only. |
| itemReview: | site:"amazon.com" AND itemReview:("Xbox One") | Track reviews of a specific product by product name (on a specific site, such as Amazon if required). |
| rating: | rating:3 rating:\[3 TO 5\] | Track reviews with a rating of 3-star rating only. Track reviews with a rating between 3 and 5 stars only. |
| minuteOfDay: | minuteOfDay:\[1110 TO 1140\] | Track mentions published within a specific range of minutes on the selected date\*. The example shows the minutes representation of 18:30 TO 19:00. \*Please note this is based on UTC. |
| pubType: | pubType:MY\_MENTIONS | Specific to custom content sources. The name is user-defined and will return mentions uploaded via the Content Upload API under that user-defined name only. In this example only mentions uploaded under the 'MY\_MENTIONS' pubType will be returned. |
| publisherSubType: | publisherSubType:IMAGE publisherSubType:VIDEO | Track Instagram posts that include images only. Track Instagram posts that include videos only. |
| publication: | publication:"MSN UK" | Returns mentions from news sources where a publication name is given rather than a site name. _Note: This component is only compatible with [rules](https://consumer-research-help.brandwatch.com/hc/en-us/articles/4402943546897) and [dashboard searches](https://consumer-research-help.brandwatch.com/hc/en-us/articles/10640372084893), not queries_ |
| redditAuthorFlair: | redditAuthorFlair:PhD | Will find mentions of posts and comments where Author Flair has been assigned on Reddit. Available on mentions created from September 17, 2024. |
| redditPostFlair: | redditPostFlair:Advice | Will find mentions of posts and comments where Post Flair has been assigned on Reddit. Available on mentions created from September 17, 2024. |
| redditSpoiler: | redditSpoiler:true | Will find mentions of posts and comments that have been marked as containing spoiler information on Reddit. Available on mentions created from September 17, 2024. |
| sensitiveContent: | sensitiveContent:true | Will find mentions of posts and comments that have been flagged as sensitive content on X (Twitter.) Available on mentions created from October 24, 2024. |
| subreddit: | subreddit: nba or Ballers | Return all posts belonging to the named subreddits. |
| subredditNSFW: | subredditNSFW:true | Will find mentions from subreddits that have been flagged as NSFW. Available on mentions created from September 17, 2024. |
| subredditTopics: | subredditTopics:“Food & Recipes” | Return all posts which match the human-readable subreddit topics. Available on mentions created from September 17, 2024. |
| topLevelDomain: | topLevelDomain:com OR topLevelDomain:org | Track mentions from all sites ending in '.com' or '.org'. |
| weblogTitle: | weblogTitle:feelslikegold | Track mentions created by a specific Reddit or forum author (in this example all Reddit and forum posts created by 'feelslikegold'). |

Operators are compatible for use across queries, rules and dashboard searches.

Should you note any unexpected behavior when attempting to use an operator in a rule or dashboard search, we'd recommend resetting your query data. This will only be required for older queries that you have been running in your account for some time. The datasets of newly created queries will be unaffected.
The reset is required to apply updates to the older dataset allowing for additional search capabilities that were not originally available.

---

## Guide to query writing with advanced operators

---

## Special characters

Please note that the following search terms do not require the use of dedicated operators and simply need to be written out as normal in the query string:

- Hashtags (e.g. #MondayMotivation)
- @mentions (e.g. @brandwatch)
- Emojis
- % : | # / ! ?
- currency symbols

_Note: If a keyword includes an apostrophe such as "wendy's" you will gather a higher volume of mentions by including both "wendy's" and "wendy s" in your query string._
