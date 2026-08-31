# ReleaseData

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**i32**> | ID of the asset release. | [optional]
**version** | Option<**String**> | Display name of the release.  **Note:** This is not a unique identifier, use the ID instead. | [optional]
**stable** | Option<**bool**> | True if this is a stable release of the asset. | [optional]
**size** | Option<**f32**> | Size of the release in MB. | [optional]
**created** | Option<[**String**](String.md)> | Date when the release was created. | [optional]
**min_godot_version** | Option<**String**> | Minimum Godot Version. | [optional]
**max_godot_version** | Option<**String**> | Maximum Godot Version. | [optional]
**notes** | Option<**String**> | Release notes. | [optional]
**changes_html** | Option<**String**> | Changelog for the version rendered as HTML. | [optional]
**changes_bbcode** | Option<**String**> | Changelog for the version rendered as BBCode. | [optional]
**download_url** | Option<**String**> | URL pointing to a downloadable file. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


