#pragma once

#include <QImage>
#include <QPainter>
#include <algorithm>

// Some mods put an opaque picture/logo in outline.png. Do not stroke its
// rectangular alpha footprint. Test the visible bounds so padding is ignored.
inline bool rectangularOutlineImage(const QImage &image)
{
    int left = image.width(), top = image.height(), right = -1, bottom = -1;
    qint64 visible = 0;
    for (int y = 0; y < image.height(); ++y) {
        const auto *row = reinterpret_cast<const QRgb *>(image.constScanLine(y));
        for (int x = 0; x < image.width(); ++x) {
            if (qAlpha(row[x]) <= 8) continue;
            ++visible;
            left = std::min(left, x); right = std::max(right, x);
            top = std::min(top, y); bottom = std::max(bottom, y);
        }
    }
    const int width = right - left + 1, height = bottom - top + 1;
    // Allow a small amount of alpha loss from resampling rounded corners.
    return width >= 3 && height >= 3
        && visible * 100 >= qint64(width) * height * 98;
}

// Called on scaled thumbnails in an image worker, never on the GUI thread.
inline QImage outlinedTrackImage(const QImage &source)
{
    if (source.isNull()) return {};
    constexpr int radius = 2;
    const QImage image = source.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    if (rectangularOutlineImage(image)) return source;
    QImage result(image.size() + QSize(radius * 2, radius * 2), QImage::Format_ARGB32_Premultiplied);
    result.fill(Qt::transparent);
    // Dilate alpha, not RGB: transparent white pixels must not create a halo.
    for (int y = 0; y < result.height(); ++y) {
        auto *row = reinterpret_cast<QRgb *>(result.scanLine(y));
        for (int x = 0; x < result.width(); ++x) {
            int alpha = 0;
            for (int dy = -radius; dy <= radius; ++dy) {
                const int sy = y - radius + dy;
                if (sy < 0 || sy >= image.height()) continue;
                const auto *sourceRow = reinterpret_cast<const QRgb *>(image.constScanLine(sy));
                for (int dx = -radius; dx <= radius; ++dx) {
                    if (dx * dx + dy * dy > radius * radius) continue;
                    const int sx = x - radius + dx;
                    if (sx >= 0 && sx < image.width()) alpha = std::max(alpha, qAlpha(sourceRow[sx]));
                }
            }
            row[x] = qRgba(0, 0, 0, alpha);
        }
    }
    QPainter painter(&result);
    painter.drawImage(radius, radius, image);
    return result;
}
