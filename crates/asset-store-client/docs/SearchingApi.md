# \SearchingApi

All URIs are relative to *https://store.godotengine.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_v1_search_autocomplete_get**](SearchingApi.md#api_v1_search_autocomplete_get) | **GET** /api/v1/search/autocomplete/ | Fetch suggestions for completing the provided query
[**api_v1_search_query_get**](SearchingApi.md#api_v1_search_query_get) | **GET** /api/v1/search/query/ | Search for assets



## api_v1_search_autocomplete_get

> Vec<models::AutocompleteResult> api_v1_search_autocomplete_get(query, layouts)
Fetch suggestions for completing the provided query

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**query** | **String** | The string that should be completed | [required] |
**layouts** | Option<[**Vec<String>**](String.md)> | Additional keyboard layouts for detecting typing errors..  This will check for typing errors by allowing keys from one layout to be interpreted as if they were pressed on the other layout at the same physical location on the keyboard.  The layouts us and uk are aways used as the main language of the store is english. |  |

### Return type

[**Vec<models::AutocompleteResult>**](AutocompleteResult.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## api_v1_search_query_get

> models::SearchResults api_v1_search_query_get(query, featured_only, require_release, stable_only, compatibility, sort, licenses, scroll, page, batch_size)
Search for assets

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**query** | **String** | The search query to perform. Tags can be specified by using the `#tag` syntax. | [required] |
**featured_only** | Option<**bool**> | Limit the result set to currently featured assets. |  |[default to false]
**require_release** | Option<**bool**> | Only include results that have a release. |  |[default to true]
**stable_only** | Option<**bool**> | Only include results that have a stable release.  **Note:** Parameter will be ignored when require_release is set to false! |  |[default to true]
**compatibility** | Option<**String**> | Only show results which provide a download for the specified Godot version.  **Note:** will be ignored used when require_release set to false true! |  |
**sort** | Option<**String**> | Determines the order of the results. |  |[default to relevance]
**licenses** | Option<[**Vec<String>**](String.md)> | Limits the results to assets that use one of the given licenses. |  |
**scroll** | Option<**String**> | Each response to a search request will include a `scroll` token. Providing it again on subsequent requests will fetch more results. This works like pagination, except that you specify the `scroll` token instead of an offset. You must use the same request parameters as you used for the initial request when using the `scroll` token to fetch further results!  Scroll tokens take precendence over other pagination options. |  |[default to null]
**page** | Option<**i32**> | Return a specific page.  **Note:** Parameter will be ignored when a scroll token is given. |  |[default to 1]
**batch_size** | Option<**i32**> | Amount of assets returned per page or scroll request. |  |[default to 24]

### Return type

[**models::SearchResults**](SearchResults.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

