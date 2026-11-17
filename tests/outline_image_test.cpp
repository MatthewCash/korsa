#include "outline_image.h"
#include <cassert>

int main()
{
    assert(outlinedTrackImage(QImage()).isNull());
    QImage image(5, 5, QImage::Format_ARGB32);
    image.fill(qRgba(255, 255, 255, 0));
    auto blank = outlinedTrackImage(image);
    for (int y = 0; y < blank.height(); ++y)
        for (int x = 0; x < blank.width(); ++x) assert(qAlpha(blank.pixel(x, y)) == 0);
    image.setPixel(2, 2, qRgba(255, 255, 255, 255));
    const auto result = outlinedTrackImage(image);
    assert(result.size() == QSize(9, 9));
    assert(result.pixelColor(4, 4) == QColor(Qt::white));
    assert(result.pixelColor(2, 4) == QColor(Qt::black));
    assert(qAlpha(result.pixel(1, 4)) == 0);
    assert(qAlpha(image.pixel(0, 0)) == 0); // Source is unchanged.
    image.setPixel(2, 2, qRgba(255, 255, 255, 100));
    assert(qAlpha(outlinedTrackImage(image).pixel(2, 4)) == 100);

    QImage rectangle(12, 8, QImage::Format_ARGB32);
    rectangle.fill(Qt::white);
    assert(outlinedTrackImage(rectangle) == rectangle);
    QImage padded(20, 20, QImage::Format_ARGB32);
    padded.fill(Qt::transparent);
    { QPainter painter(&padded); painter.fillRect(4, 5, 10, 8, Qt::white); }
    assert(outlinedTrackImage(padded) == padded);
    QImage track(20, 20, QImage::Format_ARGB32);
    track.fill(Qt::transparent);
    { QPainter painter(&track); painter.setPen(Qt::white); painter.drawEllipse(4, 4, 10, 10); }
    assert(!rectangularOutlineImage(track));
    assert(outlinedTrackImage(track).size() == QSize(24, 24));
}
