# rates_api

All URIs are relative to *http://localhost18080*

Method | HTTP request | Description
------------- | ------------- | -------------
****](rates_api.md#) | **POST** /rates/{pair} | レートを新規登録します


# ****
> models::PostSuccess (pair, rate)
レートを新規登録します

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **pair** | **String**| 通貨ペア | 
  **rate** | [**Rate**](Rate.md)|  | 

### Return type

[**models::PostSuccess**](PostSuccess.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

