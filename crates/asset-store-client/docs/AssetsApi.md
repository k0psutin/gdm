# \AssetsApi

All URIs are relative to *https://store.godotengine.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**api_v1_assets_get**](AssetsApi.md#api_v1_assets_get) | **GET** /api/v1/assets/ | Get a list of assets
[**api_v1_assets_publisher_slug_asset_slug_get**](AssetsApi.md#api_v1_assets_publisher_slug_asset_slug_get) | **GET** /api/v1/assets/{publisher_slug}/{asset_slug}/ | Get asset details



## api_v1_assets_get

> Vec<models::AssetData> api_v1_assets_get(featured_only, require_release, stable_only, compatibility, tag, page, page_size)
Get a list of assets

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**featured_only** | Option<**bool**> | Limit the result set to currently featured assets. |  |[default to false]
**require_release** | Option<**bool**> | Only include results that have a release. |  |[default to true]
**stable_only** | Option<**bool**> | Only include results that have a stable release.  **Note:** Parameter will be ignored when require_release is set to false! |  |[default to true]
**compatibility** | Option<**String**> | Only show results which provide a download for the specified Godot version.  **Note:** will be ignored used when require_release set to false true! |  |
**tag** | Option<**String**> | Filter the results by the given tag slug. |  |
**page** | Option<**i32**> |  |  |[default to 1]
**page_size** | Option<**i32**> |  |  |[default to 24]

### Return type

[**Vec<models::AssetData>**](AssetData.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## api_v1_assets_publisher_slug_asset_slug_get

> models::AssetDataDetailed api_v1_assets_publisher_slug_asset_slug_get(publisher_slug, asset_slug)
Get asset details

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**publisher_slug** | **String** | Slug that uniquely identifies the publisher | [required] |
**asset_slug** | **String** | Slug that uniquely identifies the asset | [required] |

### Return type

[**models::AssetDataDetailed**](AssetDataDetailed.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

