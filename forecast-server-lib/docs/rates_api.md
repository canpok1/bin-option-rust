# rates_api

All URIs are relative to *http://localhost:8082*

Method | HTTP request | Description
------------- | ------------- | -------------
****](rates_api.md#) | **GET** /forecast/after30min/{rateId}/{modelNo} | 30分後の予想を取得します
****](rates_api.md#) | **POST** /rates | レート履歴を新規登録します


# ****
> models::ForecastAfter30minRateIdModelNoGet200Response (rate_id, model_no)
30分後の予想を取得します

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **rate_id** | **String**| レート履歴ID | 
  **model_no** | **i32**| モデルNo | 

### Return type

[**models::ForecastAfter30minRateIdModelNoGet200Response**](_forecast_after30min__rateId___modelNo__get_200_response.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# ****
> models::RatesPost201Response (history)
レート履歴を新規登録します

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **history** | [**History**](History.md)|  | 

### Return type

[**models::RatesPost201Response**](_rates_post_201_response.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

