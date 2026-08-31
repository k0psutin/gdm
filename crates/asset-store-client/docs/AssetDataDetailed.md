# AssetDataDetailed

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
**body_html** | Option<**String**> | Content of the asset page rendered as HTML. | [optional]
**body_bbcode** | Option<**String**> | Content of the asset page rendered as BBCode. | [optional]
**donation_text** | Option<**String**> | Call to action that can be displayed on the donation button. | [optional]
**donation_url** | Option<**String**> | URL to an external donation page for the asset.  **Note:** The target page is neither hosted by nor affiliated with the Godot Foundation. | [optional]
**source** | Option<**String**> | URL leading to an external page hosting the source code.  **Note:** The target page is neither hosted by nor affiliated with the Godot Foundation. | [optional]
**featured_thumbnail** | Option<**serde_json::Value**> |  | [optional]
**media** | Option<**serde_json::Value**> |  | [optional]
**video_id** | Option<**String**> | URL to an external video.  **Note:** The target page is neither hosted by nor affiliated with the Godot Foundation. | [optional]
**created** | Option<**String**> | Date time in iso format specifying the creation date. | [optional]
**last_updated** | Option<**String**> | Date time in iso format specifying the last modification date. | [optional]
**featured** | Option<**bool**> | True if the asset is currently featured. | [optional]
**video_playback_url** | Option<**serde_json::Value**> |  | [optional]
**video_embed_url** | Option<**serde_json::Value**> |  | [optional]
**video_thumbnail_url** | Option<**serde_json::Value**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


