# SearchResults

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**count** | Option<**String**> | Total amount of results available (you may need to use `scroll` to fetch all). | [optional]
**hits** | Option<[**Vec<models::SearchResultHit>**](SearchResultHit.md)> | List of search results including highlighted passages. | [optional]
**tag_filters** | Option<[**Vec<models::SearchResultTag>**](SearchResultTag.md)> | Tags that can be used to narrow down the search results further. Will be empty if a scroll token was provided. (Tag filters will not change when loading more results for the same query) | [optional]
**scroll** | Option<**String**> | Use this token on the next `/search` request to fetch more results for the same query. You shall not change any other request parameters when providing a scroll token! | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


