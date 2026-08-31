# \TagsApi

All URIs are relative to *https://store.godotengine.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_v1_tags_get**](TagsApi.md#api_v1_tags_get) | **GET** /api/v1/tags/ | Get a list of tags



## api_v1_tags_get

> Vec<models::TagData> api_v1_tags_get(featured_only)
Get a list of tags

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**featured_only** | Option<**bool**> | Limit the result set to currently featured tags. |  |[default to false]

### Return type

[**Vec<models::TagData>**](TagData.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

