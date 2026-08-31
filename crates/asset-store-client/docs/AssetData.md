# AssetData

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**slug** | Option<**String**> | Unique identifier for the Asset. | [optional]
**publisher** | Option<[**models::PublisherData**](PublisherData.md)> | Publisher of this asset | [optional]
**name** | Option<**String**> | Human readable display name. | [optional]
**description** | Option<**String**> | Small summary of the asset page. | [optional]
**price_cent** | Option<**i32**> | Price in cents (Euro). | [optional]
**license_type** | Option<**String**> | Name of the license. | [optional]
**license_url** | Option<**String**> | URL to the exact terms of the license. | [optional]
**thumbnail** | Option<**serde_json::Value**> |  | [optional]
**reviews_score** | Option<**i32**> | Sum of upvotes (+1) and downvotes (-1). | [optional]
**tags** | Option<[**Vec<models::TagData>**](TagData.md)> | Tags that are associated with this asset. | [optional]
**store_url** | Option<**serde_json::Value**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


