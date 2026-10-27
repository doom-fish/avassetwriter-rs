#include "AVAssetWriterABI.h"
#import <AVFoundation/AVFoundation.h>
#import <CoreMedia/CoreMedia.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

AVAssetWriter * _Nullable AVWXMakeWriter(
    NSURL *url,
    AVFileType fileType,
    NSString * _Nullable * _Nullable message
);

AVAssetWriterInput * _Nullable AVWXMakeInput(
    AVMediaType mediaType,
    NSDictionary<NSString *, id> * _Nullable outputSettings,
    CMFormatDescriptionRef _Nullable sourceFormatHint,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXAddInput(
    AVAssetWriter *writer,
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXAddInputGroup(
    AVAssetWriter *writer,
    AVAssetWriterInputGroup *group,
    NSString * _Nullable * _Nullable message
);

NSInteger AVWXCanAddTrackAssociation(
    AVAssetWriterInput *input,
    AVAssetWriterInput *other,
    NSString *associationType,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXAddTrackAssociation(
    AVAssetWriterInput *input,
    AVAssetWriterInput *other,
    NSString *associationType,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXSetLanguageCode(
    AVAssetWriterInput *input,
    NSString * _Nullable languageCode,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXSetExtendedLanguageTag(
    AVAssetWriterInput *input,
    NSString * _Nullable extendedLanguageTag,
    NSString * _Nullable * _Nullable message
);

AVAssetWriterInputPixelBufferAdaptor * _Nullable AVWXMakePixelBufferAdaptor(
    AVAssetWriterInput *input,
    NSDictionary<NSString *, id> * _Nullable sourcePixelBufferAttributes,
    NSString * _Nullable * _Nullable message
);

AVAssetWriterInputMetadataAdaptor * _Nullable AVWXMakeMetadataAdaptor(
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
);

AVAssetWriterInputCaptionAdaptor * _Nullable AVWXMakeCaptionAdaptor(
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
);

NSInteger AVWXAppendSampleBuffer(
    AVAssetWriterInput *input,
    CMSampleBufferRef sampleBuffer,
    NSString * _Nullable * _Nullable message
);

NSInteger AVWXAppendPixelBuffer(
    AVAssetWriterInputPixelBufferAdaptor *adaptor,
    CVPixelBufferRef pixelBuffer,
    CMTime presentationTime,
    NSString * _Nullable * _Nullable message
);

CVPixelBufferPoolRef _Nullable AVWXPixelBufferPool(
    AVAssetWriterInputPixelBufferAdaptor *adaptor,
    NSString * _Nullable * _Nullable message
) CF_RETURNS_NOT_RETAINED;

AVAssetWriterInputTaggedPixelBufferGroupAdaptor * _Nullable AVWXMakeTaggedPixelBufferGroupAdaptor(
    AVAssetWriterInput *input,
    NSDictionary<NSString *, id> * _Nullable sourcePixelBufferAttributes,
    NSString * _Nullable * _Nullable message
) API_AVAILABLE(macos(14.0));

NSInteger AVWXAppendCaption(
    AVAssetWriterInputCaptionAdaptor *adaptor,
    AVCaption *caption,
    NSString * _Nullable * _Nullable message
);

NSInteger AVWXAppendCaptionGroup(
    AVAssetWriterInputCaptionAdaptor *adaptor,
    AVCaptionGroup *captionGroup,
    NSString * _Nullable * _Nullable message
);

BOOL AVWXSetCaptionRegion(
    AVMutableCaption *caption,
    AVCaptionRegion *region,
    NSString * _Nullable * _Nullable message
);

AVCaptionGroup * _Nullable AVWXMakeCaptionGroup(
    NSArray<AVCaption *> *captions,
    CMTimeRange timeRange,
    NSString * _Nullable * _Nullable message
);

AVCaptionFormatConformer * _Nullable AVWXMakeCaptionFormatConformer(
    NSDictionary<AVCaptionSettingsKey, id> *conversionSettings,
    NSString * _Nullable * _Nullable message
);

AVCaptionConversionValidator * _Nullable AVWXMakeCaptionConversionValidator(
    NSArray<AVCaption *> *captions,
    CMTimeRange timeRange,
    NSDictionary<AVCaptionSettingsKey, id> *conversionSettings,
    NSString * _Nullable * _Nullable message
);

NS_ASSUME_NONNULL_END
