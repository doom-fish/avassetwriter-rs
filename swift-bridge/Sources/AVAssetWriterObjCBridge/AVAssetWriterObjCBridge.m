#import "AVAssetWriterObjCBridge.h"

static void AVWXStoreException(NSException *exception, NSString * _Nullable * _Nullable message) {
    if (message != NULL) {
        *message = exception.reason ?: exception.name;
    }
}

AVAssetWriter * _Nullable AVWXMakeWriter(
    NSURL *url,
    AVFileType fileType,
    NSString * _Nullable * _Nullable message
) {
    @try {
        NSError *error = nil;
        AVAssetWriter *writer = [[AVAssetWriter alloc] initWithURL:url fileType:fileType error:&error];
        if (writer == nil && message != NULL) {
            *message = error.localizedDescription ?: @"AVAssetWriter init failed";
        }
        return writer;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

AVAssetWriterInput * _Nullable AVWXMakeInput(
    AVMediaType mediaType,
    NSDictionary<NSString *, id> * _Nullable outputSettings,
    CMFormatDescriptionRef _Nullable sourceFormatHint,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVAssetWriterInput alloc] initWithMediaType:mediaType
                                              outputSettings:outputSettings
                                            sourceFormatHint:sourceFormatHint];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

BOOL AVWXAddInput(
    AVAssetWriter *writer,
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
) {
    @try {
        if (![writer canAddInput:input]) {
            if (message != NULL) {
                *message = [NSString stringWithFormat:@"writer cannot add input (status=%ld)", (long)writer.status];
            }
            return NO;
        }
        [writer addInput:input];
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

BOOL AVWXAddInputGroup(
    AVAssetWriter *writer,
    AVAssetWriterInputGroup *group,
    NSString * _Nullable * _Nullable message
) {
    @try {
        if (![writer canAddInputGroup:group]) {
            if (message != NULL) {
                *message = @"writer cannot add input group";
            }
            return NO;
        }
        [writer addInputGroup:group];
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

NSInteger AVWXCanAddTrackAssociation(
    AVAssetWriterInput *input,
    AVAssetWriterInput *other,
    NSString *associationType,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [input canAddTrackAssociationWithTrackOfInput:other type:associationType] ? 1 : 0;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return -1;
    }
}

BOOL AVWXAddTrackAssociation(
    AVAssetWriterInput *input,
    AVAssetWriterInput *other,
    NSString *associationType,
    NSString * _Nullable * _Nullable message
) {
    @try {
        if (![input canAddTrackAssociationWithTrackOfInput:other type:associationType]) {
            if (message != NULL) {
                *message = [NSString stringWithFormat:@"cannot add track association '%@'", associationType];
            }
            return NO;
        }
        [input addTrackAssociationWithTrackOfInput:other type:associationType];
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

BOOL AVWXSetLanguageCode(
    AVAssetWriterInput *input,
    NSString * _Nullable languageCode,
    NSString * _Nullable * _Nullable message
) {
    @try {
        input.languageCode = languageCode;
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

BOOL AVWXSetExtendedLanguageTag(
    AVAssetWriterInput *input,
    NSString * _Nullable extendedLanguageTag,
    NSString * _Nullable * _Nullable message
) {
    @try {
        input.extendedLanguageTag = extendedLanguageTag;
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

AVAssetWriterInputPixelBufferAdaptor * _Nullable AVWXMakePixelBufferAdaptor(
    AVAssetWriterInput *input,
    NSDictionary<NSString *, id> * _Nullable sourcePixelBufferAttributes,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVAssetWriterInputPixelBufferAdaptor alloc] initWithAssetWriterInput:input
                                                           sourcePixelBufferAttributes:sourcePixelBufferAttributes];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

AVAssetWriterInputMetadataAdaptor * _Nullable AVWXMakeMetadataAdaptor(
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVAssetWriterInputMetadataAdaptor alloc] initWithAssetWriterInput:input];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

AVAssetWriterInputCaptionAdaptor * _Nullable AVWXMakeCaptionAdaptor(
    AVAssetWriterInput *input,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVAssetWriterInputCaptionAdaptor alloc] initWithAssetWriterInput:input];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

NSInteger AVWXAppendSampleBuffer(
    AVAssetWriterInput *input,
    CMSampleBufferRef sampleBuffer,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [input appendSampleBuffer:sampleBuffer] ? 1 : 0;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return -1;
    }
}

NSInteger AVWXAppendPixelBuffer(
    AVAssetWriterInputPixelBufferAdaptor *adaptor,
    CVPixelBufferRef pixelBuffer,
    CMTime presentationTime,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [adaptor appendPixelBuffer:pixelBuffer withPresentationTime:presentationTime] ? 1 : 0;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return -1;
    }
}

CVPixelBufferPoolRef _Nullable AVWXPixelBufferPool(
    AVAssetWriterInputPixelBufferAdaptor *adaptor,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return adaptor.pixelBufferPool;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NULL;
    }
}

AVAssetWriterInputTaggedPixelBufferGroupAdaptor * _Nullable AVWXMakeTaggedPixelBufferGroupAdaptor(
    AVAssetWriterInput *input,
    NSDictionary<NSString *, id> * _Nullable sourcePixelBufferAttributes,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVAssetWriterInputTaggedPixelBufferGroupAdaptor alloc]
            initWithAssetWriterInput:input
          sourcePixelBufferAttributes:sourcePixelBufferAttributes];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

NSInteger AVWXAppendCaption(
    AVAssetWriterInputCaptionAdaptor *adaptor,
    AVCaption *caption,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [adaptor appendCaption:caption] ? 1 : 0;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return -1;
    }
}

NSInteger AVWXAppendCaptionGroup(
    AVAssetWriterInputCaptionAdaptor *adaptor,
    AVCaptionGroup *captionGroup,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [adaptor appendCaptionGroup:captionGroup] ? 1 : 0;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return -1;
    }
}

BOOL AVWXSetCaptionRegion(
    AVMutableCaption *caption,
    AVCaptionRegion *region,
    NSString * _Nullable * _Nullable message
) {
    @try {
        caption.region = region;
        return YES;
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return NO;
    }
}

AVCaptionGroup * _Nullable AVWXMakeCaptionGroup(
    NSArray<AVCaption *> *captions,
    CMTimeRange timeRange,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVCaptionGroup alloc] initWithCaptions:captions timeRange:timeRange];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

AVCaptionFormatConformer * _Nullable AVWXMakeCaptionFormatConformer(
    NSDictionary<AVCaptionSettingsKey, id> *conversionSettings,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVCaptionFormatConformer alloc] initWithConversionSettings:conversionSettings];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}

AVCaptionConversionValidator * _Nullable AVWXMakeCaptionConversionValidator(
    NSArray<AVCaption *> *captions,
    CMTimeRange timeRange,
    NSDictionary<AVCaptionSettingsKey, id> *conversionSettings,
    NSString * _Nullable * _Nullable message
) {
    @try {
        return [[AVCaptionConversionValidator alloc] initWithCaptions:captions
                                                            timeRange:timeRange
                                                   conversionSettings:conversionSettings];
    } @catch (NSException *exception) {
        AVWXStoreException(exception, message);
        return nil;
    }
}
