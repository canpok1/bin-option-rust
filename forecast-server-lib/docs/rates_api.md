# rates_api

All URIs are relative to *http://localhost28080*

Method | HTTP request | Description
------------- | ------------- | -------------
****](rates_api.md#) | **GET** /forecast/after5min/{historyId} | 5分後の予想を取得します
****](rates_api.md#) | **POST** /histories | レート履歴を新規登録します


# ****
> models::ForecastAfter5minHistoryIdGet200Response (history_id)
5分後の予想を取得します

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **history_id** | **String**| レート履歴ID | 

### Return type

[**models::ForecastAfter5minHistoryIdGet200Response**](_forecast_after5min__historyId__get_200_response.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# ****
> models::HistoriesPost201Response (history)
レート履歴を新規登録します

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **history** | [**History**](History.md)|  | 

### Return type

[**models::HistoriesPost201Response**](_histories_post_201_response.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

