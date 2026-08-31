# \ReleasesApi

All URIs are relative to *https://store.godotengine.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_v1_releases_publisher_slug_asset_slug_get**](ReleasesApi.md#api_v1_releases_publisher_slug_asset_slug_get) | **GET** /api/v1/releases/{publisher_slug}/{asset_slug}/ | Get a list of available downloads for an asset



## api_v1_releases_publisher_slug_asset_slug_get

> Vec<models::ReleaseData> api_v1_releases_publisher_slug_asset_slug_get(publisher_slug, asset_slug, stable_only, compatibility)
Get a list of available downloads for an asset

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**publisher_slug** | **String** | Slug that uniquely identifies the publisher | [required] |
**asset_slug** | **String** | Slug that uniquely identifies the asset | [required] |
**stable_only** | Option<**bool**> | Only include stable releases. |  |[default to true]
**compatibility** | Option<**String**> | Limit to releases that are compatible with the specified Godot version. |  |

### Return type

[**Vec<models::ReleaseData>**](ReleaseData.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

