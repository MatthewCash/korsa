#include "widgets.h"

#include "korsa/src/backend.cxxqt.h"
#include "window_effects.h"

#include <QApplication>
#include <QCheckBox>
#include <QComboBox>
#include <QDateTime>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDesktopServices>
#include <QDoubleSpinBox>
#include <QFileInfo>
#include <QFileDialog>
#include <QFormLayout>
#include <QFrame>
#include <QGraphicsScene>
#include <QGraphicsItem>
#include <QGraphicsView>
#include <QGroupBox>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QIcon>
#include <QImageReader>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QMap>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QProgressBar>
#include <QPainter>
#include <QPushButton>
#include <QPixmapCache>
#include <QResizeEvent>
#include <QScrollArea>
#include <QSet>
#include <QSignalBlocker>
#include <QShortcut>
#include <QSlider>
#include <QSpinBox>
#include <QSplitter>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabWidget>
#include <QTabBar>
#include <QTableWidget>
#include <QTimeEdit>
#include <QTimer>
#include <QToolBar>
#include <QToolButton>
#include <QUrl>
#include <QVBoxLayout>
#include <algorithm>

namespace {

QString textProperty(QObject *object, const char *name)
{
    return object->property(name).toString();
}

bool boolProperty(QObject *object, const char *name)
{
    return object->property(name).toBool();
}

void invoke(QObject *backend, const char *method)
{
    QMetaObject::invokeMethod(backend, method, Qt::AutoConnection);
}

void invokeOption(QObject *backend, const char *method, const QString &file,
                  const QString &section, const QString &key, const QString &value)
{
    QMetaObject::invokeMethod(backend, method, Qt::AutoConnection,
                              Q_ARG(QString, file), Q_ARG(QString, section),
                              Q_ARG(QString, key), Q_ARG(QString, value));
}

QPixmap pixmapFromUrl(const QString &value, const QSize &size)
{
    const QString path = QUrl(value).toLocalFile();
    if (path.isEmpty()) {
        return {};
    }
    const QString cacheKey = path + QStringLiteral("@")
        + QString::number(size.width()) + QStringLiteral("x") + QString::number(size.height());
    QPixmap cached;
    if (QPixmapCache::find(cacheKey, &cached)) {
        return cached;
    }
    QPixmap pixmap(path);
    if (!pixmap.isNull()) {
        pixmap = pixmap.scaled(size, Qt::KeepAspectRatio, Qt::SmoothTransformation);
        QPixmapCache::insert(cacheKey, pixmap);
    }
    return pixmap;
}

QPixmap trackPixmap(const QJsonObject &track, const QSize &size)
{
    QPixmap preview = pixmapFromUrl(track.value("preview").toString(), size);
    const QSize outlineSize(qRound(size.width() * 0.42), qRound(size.height() * 0.72));
    const QPixmap outline = pixmapFromUrl(track.value("outline").toString(), outlineSize);
    if (outline.isNull()) return preview;

    QPixmap combined(size);
    combined.fill(Qt::transparent);
    QPainter painter(&combined);
    if (!preview.isNull()) {
        painter.drawPixmap((size.width() - preview.width()) / 2,
                           (size.height() - preview.height()) / 2, preview);
    }
    painter.drawPixmap(size.width() - outline.width() - 6,
                       size.height() - outline.height() - 6, outline);
    return combined;
}

QString humanizeTrackId(QString id)
{
    if (id.startsWith(QStringLiteral("ks_"))) id.remove(0, 3);
    id.replace('_', ' ');
    bool capitalize = true;
    for (qsizetype index = 0; index < id.size(); ++index) {
        const QChar character = id.at(index);
        if (capitalize && character.isLetter()) id[index] = character.toUpper();
        capitalize = character.isSpace();
    }
    return id;
}

QString sharedTrackName(const QJsonArray &layouts, const QString &fallback)
{
    if (layouts.isEmpty()) return fallback;
    QString prefix = layouts.first().toObject().value("name").toString();
    for (const auto &value : layouts) {
        const QString name = value.toObject().value("name").toString();
        int common = 0;
        while (common < prefix.size() && common < name.size() && prefix.at(common) == name.at(common)) ++common;
        prefix.truncate(common);
    }
    while (!prefix.isEmpty() && (prefix.back().isSpace() || QStringLiteral("-/|:(").contains(prefix.back()))) prefix.chop(1);
    return prefix.size() >= 4 ? prefix : fallback;
}

double catalogNumber(const QString &value)
{
    QString number;
    for (const QChar character : value) {
        if (character.isDigit() || (character == '.' && !number.contains('.'))) number.append(character);
        else if (!number.isEmpty()) break;
    }
    return number.toDouble();
}

QScrollArea *scrollPage(QWidget *content)
{
    auto *area = new QScrollArea;
    area->setWidgetResizable(true);
    area->setFrameShape(QFrame::NoFrame);
    area->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    content->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    area->setWidget(content);
    return area;
}

class MainWindow final : public QMainWindow {
public:
    explicit MainWindow(Backend *backend)
        : backend_(backend)
    {
        setWindowTitle(QStringLiteral("Korsa"));
        resize(1280, 820);
        setMinimumSize(980, 650);
        buildShell();
        buildDrive();
        buildCatalog(true);
        buildCatalog(false);
        buildRace();
        buildOnline();
        buildReplays();
        buildContentManager();
        buildTools();
        buildSettings();

        poll_.setInterval(200);
        connect(&poll_, &QTimer::timeout, this, [this] { refresh(); });
        poll_.start();
        racePoll_.setInterval(1000);
        connect(&racePoll_, &QTimer::timeout, this, [this] { invoke(backend_, "refreshRaceState"); });
        racePoll_.start();
        applyTimer_.setSingleShot(true);
        applyTimer_.setInterval(220);
        connect(&applyTimer_, &QTimer::timeout, this, [this] { applyDrive(); });
        QTimer::singleShot(0, this, [this] {
            enable_aclm_blur();
            backend_->discover();
        });
    }

private:
    struct CatalogWidgets {
        QListWidget *list{};
        QLineEdit *search{};
        QComboBox *filter{};
        QComboBox *sort{};
        QCheckBox *favoritesOnly{};
        QLabel *image{};
        QLabel *title{};
        QLabel *details{};
        QComboBox *layouts{};
        QPushButton *favorite{};
        QPushButton *dashboard{};
        QPushButton *use{};
        QJsonObject selected;
        bool cars{};
    };

    struct AxisMeter {
        QProgressBar *meter{};
        int controller{-1};
        int axis{-1};
    };

    struct ButtonIndicator {
        QLabel *label{};
        int controller{-1};
        int button{-1};
    };

    void resizeEvent(QResizeEvent *event) override
    {
        QMainWindow::resizeEvent(event);
        updateDriveLayout();
    }

    void buildShell()
    {
        auto *refreshAction = new QAction(QIcon::fromTheme("view-refresh"), tr("Refresh"), this);
        refreshAction->setShortcut(QKeySequence::Refresh);
        connect(refreshAction, &QAction::triggered, this, [this] { invoke(backend_, "discover"); });
        driveAction_ = new QAction(QIcon::fromTheme("media-playback-start"), tr("Drive"), this);
        driveAction_->setShortcut(QKeySequence(QStringLiteral("Ctrl+G")));
        connect(driveAction_, &QAction::triggered, this,
                [this] { launchDrive(); });
        auto *toolbar = addToolBar(tr("Main"));
        toolbar->setMovable(false);
        toolbar->setIconSize(QSize(18, 18));
        toolbar->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        toolbar->addAction(refreshAction);
        toolbar->addAction(driveAction_);
        toolbar->hide();
        menuBar()->addMenu(tr("File"))->addAction(refreshAction);
        menuBar()->addMenu(tr("Session"))->addAction(driveAction_);
        menuBar()->hide();
        addAction(refreshAction);
        addAction(driveAction_);
        auto *toggleMenu = new QShortcut(QKeySequence(QStringLiteral("Ctrl+M")), this);
        connect(toggleMenu, &QShortcut::activated, this, [this] { menuBar()->setVisible(!menuBar()->isVisible()); });

        auto *splitter = new QSplitter;
        splitter->setHandleWidth(1);
        navigation_ = new QListWidget;
        navigation_->setFixedWidth(158);
        navigation_->setIconSize(QSize(24, 24));
        navigation_->setSpacing(2);
        navigation_->setFrameShape(QFrame::NoFrame);
        navigation_->setAutoFillBackground(false);
        navigation_->viewport()->setAutoFillBackground(false);
        QPalette navigationPalette = navigation_->palette();
        navigationPalette.setBrush(QPalette::Base, Qt::transparent);
        navigationPalette.setBrush(QPalette::AlternateBase, Qt::transparent);
        navigation_->setPalette(navigationPalette);
        const QList<QPair<QString, QString>> pages = {
            {tr("Drive"), "media-playback-start"}, {tr("Cars"), "car"},
            {tr("Tracks"), "map-globe"}, {tr("Race Setup"), "configure"},
            {tr("Online"), "network-server"}, {tr("Replays"), "media-playback-pause"},
            {tr("Content"), "package-x-generic"}, {tr("Tools"), "applications-engineering"},
            {tr("Settings"), "preferences-system"}};
        for (const auto &[label, icon] : pages) {
            auto *item = new QListWidgetItem(QIcon::fromTheme(icon), label);
            item->setSizeHint(QSize(150, 44));
            navigation_->addItem(item);
        }
        stack_ = new QStackedWidget;
        splitter->addWidget(navigation_);
        splitter->addWidget(stack_);
        splitter->setStretchFactor(1, 1);
        setCentralWidget(splitter);
        connect(navigation_, &QListWidget::currentRowChanged, stack_, &QStackedWidget::setCurrentIndex);
        navigation_->setCurrentRow(0);

        statusLabel_ = new QLabel(tr("Ready"));
        cspLabel_ = new QLabel;
        busy_ = new QProgressBar;
        busy_->setRange(0, 0);
        busy_->setMaximumWidth(100);
        busy_->hide();
        statusBar()->addWidget(statusLabel_, 1);
        statusBar()->addPermanentWidget(cspLabel_);
        statusBar()->addPermanentWidget(busy_);
    }

    void buildDrive()
    {
        auto *content = new QWidget;
        auto *layout = new QVBoxLayout(content);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(14);
        auto *header = new QHBoxLayout;
        auto *title = new QLabel(tr("Drive"));
        QFont titleFont = title->font();
        titleFont.setPointSize(titleFont.pointSize() + 8);
        titleFont.setBold(true);
        title->setFont(titleFont);
        auto *subtitle = new QLabel(tr("Configure the next session and launch Assetto Corsa"));
        subtitle->setForegroundRole(QPalette::PlaceholderText);
        auto *heading = new QVBoxLayout;
        heading->setSpacing(1);
        heading->addWidget(title);
        heading->addWidget(subtitle);
        header->addLayout(heading);
        header->addStretch();
        layout->addLayout(header);

        driveSelection_ = new QGridLayout;
        auto *selection = driveSelection_;
        selection->setSpacing(12);

        auto *carColumn = new QVBoxLayout;
        carColumn->setSpacing(4);
        carPreview_ = new QToolButton;
        carPreview_->setToolButtonStyle(Qt::ToolButtonTextUnderIcon);
        carPreview_->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
        carPreview_->setMinimumHeight(215);
        carPreview_->setIconSize(QSize(360, 155));
        connect(carPreview_, &QToolButton::clicked, this, [this] { navigation_->setCurrentRow(1); });
        carColumn->addWidget(carPreview_);
        auto *carVariantLabel = new QLabel(tr("Car variant"));
        carVariantLabel->setBuddy(carVariant_ = new QComboBox);
        carVariant_->setIconSize(QSize(144, 81));
        carVariant_->setMinimumHeight(88);
        carColumn->addWidget(carVariantLabel);
        carColumn->addWidget(carVariant_);
        auto *skinLabel = new QLabel(tr("Car livery"));
        skinLabel->setBuddy(skin_ = new QComboBox);
        skin_->setIconSize(QSize(144, 81));
        skin_->setMinimumHeight(88);
        carColumn->addWidget(skinLabel);
        carColumn->addWidget(skin_);
        driveCarPanel_ = new QWidget;
        driveCarPanel_->setLayout(carColumn);

        auto *trackColumn = new QVBoxLayout;
        trackColumn->setSpacing(4);
        trackPreview_ = new QToolButton;
        trackPreview_->setToolButtonStyle(Qt::ToolButtonTextUnderIcon);
        trackPreview_->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
        trackPreview_->setMinimumHeight(215);
        trackPreview_->setIconSize(QSize(360, 155));
        connect(trackPreview_, &QToolButton::clicked, this, [this] { navigation_->setCurrentRow(2); });
        trackColumn->addWidget(trackPreview_);
        auto *trackLayoutLabel = new QLabel(tr("Track variant"));
        trackLayoutLabel->setBuddy(trackLayout_ = new QComboBox);
        trackLayout_->setIconSize(QSize(144, 81));
        trackLayout_->setMinimumHeight(88);
        trackColumn->addWidget(trackLayoutLabel);
        trackColumn->addWidget(trackLayout_);
        driveTrackPanel_ = new QWidget;
        driveTrackPanel_->setLayout(trackColumn);

        auto *launchPanel = new QGroupBox(tr("Ready to race"));
        launchPanel->setMinimumWidth(225);
        auto *launchLayout = new QVBoxLayout(launchPanel);
        launchSummary_ = new QLabel;
        launchSummary_->setWordWrap(true);
        launchSummary_->setAlignment(Qt::AlignTop | Qt::AlignLeft);
        driveButton_ = new QPushButton(QIcon::fromTheme("media-playback-start"), tr("Drive"));
        driveButton_->setMinimumHeight(58);
        QFont launchFont = driveButton_->font(); launchFont.setBold(true); launchFont.setPointSize(launchFont.pointSize() + 2); driveButton_->setFont(launchFont);
        connect(driveButton_, &QPushButton::clicked, this, [this] { launchDrive(); });
        auto *showroom = new QPushButton(QIcon::fromTheme("applications-graphics"), tr("Showroom"));
        showroom->setMinimumHeight(42);
        connect(showroom, &QPushButton::clicked, this, [this] { invoke(backend_, "launchShowroom"); });
        launchLayout->addWidget(launchSummary_, 1);
        launchLayout->addWidget(driveButton_);
        launchLayout->addWidget(showroom);
        driveLaunchPanel_ = launchPanel;
        selection->addWidget(driveCarPanel_, 0, 0);
        selection->addWidget(driveTrackPanel_, 0, 1);
        selection->addWidget(driveLaunchPanel_, 0, 2);
        selection->setColumnStretch(0, 1);
        selection->setColumnStretch(1, 1);
        layout->addLayout(selection);

        auto *setup = new QGroupBox(tr("Quick Setup"));
        auto *form = new QFormLayout(setup);
        mode_ = new QComboBox;
        const QList<QPair<QString, QString>> modes = {{tr("Practice"), "practice"}, {tr("Qualifying"), "qualifying"},
            {tr("Race"), "race"}, {tr("Hotlap"), "hotlap"}, {tr("Time Attack"), "time_attack"},
            {tr("Drift"), "drift"}, {tr("Drag"), "drag"}};
        for (const auto &[name, value] : modes) mode_->addItem(name, value);
        opponents_ = spin(0, 63); ai_ = spin(70, 100); laps_ = spin(1, 999); duration_ = spin(0, 1440);
        penalties_ = new QCheckBox(tr("Enabled"));
        form->addRow(tr("Mode"), mode_);
        form->addRow(tr("Opponents"), sliderControl(opponents_)); form->addRow(tr("AI strength"), sliderControl(ai_));
        form->addRow(tr("Race laps"), laps_); form->addRow(tr("Duration (minutes)"), duration_);
        form->addRow(tr("Penalties"), penalties_);

        auto *conditions = new QGroupBox(tr("Conditions"));
        auto *conditionForm = new QFormLayout(conditions);
        weather_ = new QComboBox;
        time_ = new QTimeEdit;
        time_->setDisplayFormat(QStringLiteral("HH:mm"));
        air_ = spin(-40, 60); road_ = spin(-40, 100);
        conditionForm->addRow(tr("Weather"), weather_);
        auto *timeRow = new QWidget;
        driveTimeLayout_ = new QGridLayout(timeRow);
        driveTimeLayout_->setContentsMargins(0, 0, 0, 0);
        driveTimeLayout_->addWidget(time_, 0, 0);
        for (const auto &[label, hour] : QList<QPair<QString, int>>{{tr("Dawn"), 6}, {tr("Noon"), 12}, {tr("Sunset"), 18}, {tr("Midnight"), 0}}) {
            auto *button = new QToolButton; button->setText(label);
            connect(button, &QToolButton::clicked, this, [this, hour] { time_->setTime(QTime(hour, 0)); scheduleApply(); });
            driveTimeButtons_.append(button);
            driveTimeLayout_->addWidget(button, 0, driveTimeButtons_.size());
        }
        conditionForm->addRow(tr("Start time"), timeRow);
        conditionForm->addRow(tr("Air temperature"), sliderControl(air_)); conditionForm->addRow(tr("Road temperature"), sliderControl(road_));
        driveQuickPanels_ = new QBoxLayout(QBoxLayout::LeftToRight);
        driveQuickPanels_->setSpacing(12);
        driveQuickPanels_->addWidget(setup, 1);
        driveQuickPanels_->addWidget(conditions, 1);
        layout->addLayout(driveQuickPanels_);
        layout->addStretch();
        driveScroll_ = scrollPage(content);
        stack_->addWidget(driveScroll_);
        QTimer::singleShot(0, this, [this] { updateDriveLayout(); });

        for (auto *combo : {mode_, weather_}) connect(combo, &QComboBox::activated, this, [this] { scheduleApply(); });
        connect(skin_, &QComboBox::activated, this, [this] { selectedSkin_ = skin_->currentData().toString(); scheduleApply(); });
        connect(carVariant_, &QComboBox::activated, this, [this] {
            const QJsonObject variant = carVariant_->currentData().toJsonObject();
            if (variant.isEmpty()) return;
            selectedCar_ = variant.value("id").toString();
            selectedSkin_ = variant.value("skin").toString();
            scheduleApply();
        });
        connect(trackLayout_, &QComboBox::activated, this, [this] {
            const QJsonObject layout = trackLayout_->currentData().toJsonObject();
            if (!layout.isEmpty()) { selectedTrack_ = layout.value("id").toString(); scheduleApply(); }
        });
        for (auto *box : {opponents_, ai_, laps_, duration_, air_, road_}) connect(box, &QSpinBox::valueChanged, this, [this] { scheduleApply(); });
        connect(time_, &QTimeEdit::timeChanged, this, [this] { scheduleApply(); });
        connect(penalties_, &QCheckBox::toggled, this, [this] { scheduleApply(); });
    }

    void updateDriveLayout()
    {
        if (!driveScroll_) return;
        const int width = driveScroll_->viewport()->width();
        const int layoutMode = width >= 900 ? 0 : width >= 800 ? 1
            : width >= 700 ? 2 : width >= 560 ? 3 : 4;
        if (layoutMode == driveLayoutMode_) return;
        driveLayoutMode_ = layoutMode;

        driveSelection_->removeWidget(driveCarPanel_);
        driveSelection_->removeWidget(driveTrackPanel_);
        driveSelection_->removeWidget(driveLaunchPanel_);
        if (width >= 900) {
            driveSelection_->addWidget(driveCarPanel_, 0, 0);
            driveSelection_->addWidget(driveTrackPanel_, 0, 1);
            driveSelection_->addWidget(driveLaunchPanel_, 0, 2);
        } else if (width >= 560) {
            driveSelection_->addWidget(driveCarPanel_, 0, 0);
            driveSelection_->addWidget(driveTrackPanel_, 0, 1);
            driveSelection_->addWidget(driveLaunchPanel_, 1, 0, 1, 2);
        } else {
            driveSelection_->addWidget(driveCarPanel_, 0, 0);
            driveSelection_->addWidget(driveTrackPanel_, 1, 0);
            driveSelection_->addWidget(driveLaunchPanel_, 2, 0);
        }

        driveQuickPanels_->setDirection(width >= 800
            ? QBoxLayout::LeftToRight : QBoxLayout::TopToBottom);

        driveTimeLayout_->removeWidget(time_);
        for (auto *button : driveTimeButtons_) driveTimeLayout_->removeWidget(button);
        if (width >= 700) {
            driveTimeLayout_->addWidget(time_, 0, 0);
            for (int index = 0; index < driveTimeButtons_.size(); ++index)
                driveTimeLayout_->addWidget(driveTimeButtons_.at(index), 0, index + 1);
        } else {
            driveTimeLayout_->addWidget(time_, 0, 0, 1, 2);
            for (int index = 0; index < driveTimeButtons_.size(); ++index)
                driveTimeLayout_->addWidget(driveTimeButtons_.at(index), 1 + index / 2, index % 2);
        }
    }

    QSpinBox *spin(int minimum, int maximum)
    {
        auto *box = new QSpinBox;
        box->setRange(minimum, maximum);
        box->setKeyboardTracking(false);
        return box;
    }

    QWidget *sliderControl(QSpinBox *box)
    {
        auto *control = new QWidget;
        auto *layout = new QHBoxLayout(control); layout->setContentsMargins(0, 0, 0, 0);
        auto *slider = new QSlider(Qt::Horizontal); slider->setRange(box->minimum(), box->maximum()); slider->setValue(box->value());
        box->setFixedWidth(78);
        connect(slider, &QSlider::valueChanged, box, &QSpinBox::setValue);
        connect(box, &QSpinBox::valueChanged, slider, &QSlider::setValue);
        layout->addWidget(slider, 1); layout->addWidget(box);
        return control;
    }

    QWidget *sliderControl(QDoubleSpinBox *box)
    {
        auto *control = new QWidget;
        auto *layout = new QHBoxLayout(control); layout->setContentsMargins(0, 0, 0, 0);
        const int scale = box->decimals() >= 2 ? 100 : box->decimals() == 1 ? 10 : 1;
        auto *slider = new QSlider(Qt::Horizontal); slider->setRange(qRound(box->minimum() * scale), qRound(box->maximum() * scale)); slider->setValue(qRound(box->value() * scale));
        box->setFixedWidth(88);
        connect(slider, &QSlider::valueChanged, box, [box, scale](int value) { box->setValue(double(value) / scale); });
        connect(box, &QDoubleSpinBox::valueChanged, slider, [slider, scale](double value) { slider->setValue(qRound(value * scale)); });
        layout->addWidget(slider, 1); layout->addWidget(box);
        return control;
    }

    void buildCatalog(bool cars)
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *header = new QHBoxLayout;
        auto *title = new QLabel(cars ? tr("Cars") : tr("Tracks"));
        QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font);
        auto *count = new QLabel;
        count->setForegroundRole(QPalette::PlaceholderText);
        header->addWidget(title); header->addWidget(count); header->addStretch();
        auto *search = new QLineEdit;
        search->setPlaceholderText(cars ? tr("Search cars") : tr("Search tracks"));
        search->setClearButtonEnabled(true);
        search->setMaximumWidth(320);
        auto *filter = new QComboBox;
        filter->addItem(tr("All"), QString());
        if (cars) {
            filter->addItem(tr("Manual"), QStringLiteral("manual"));
            filter->addItem(tr("Semiautomatic"), QStringLiteral("semiautomatic"));
            filter->addItem(tr("Automatic"), QStringLiteral("automatic"));
            filter->addItem(tr("Sequential"), QStringLiteral("sequential"));
        }
        auto *sort = new QComboBox;
        sort->addItem(tr("Name"), QStringLiteral("name"));
        sort->addItem(tr("Author"), QStringLiteral("author"));
        if (cars) sort->addItem(tr("Power"), QStringLiteral("power"));
        sort->addItem(tr("Favorites first"), QStringLiteral("favorite"));
        auto *favoritesOnly = new QCheckBox(tr("Favorites"));
        header->addWidget(search);
        header->addWidget(filter);
        header->addWidget(sort);
        header->addWidget(favoritesOnly);
        auto *list = new QListWidget;
        list->setViewMode(QListView::IconMode);
        list->setMovement(QListView::Static);
        list->setDragDropMode(QAbstractItemView::NoDragDrop);
        list->setResizeMode(QListView::Adjust);
        list->setIconSize(QSize(190, 108));
        list->setGridSize(QSize(235, 184));
        list->setSpacing(4);
        list->setUniformItemSizes(true);
        list->setSelectionMode(QAbstractItemView::SingleSelection);
        list->setWordWrap(true);
        auto *details = new QWidget;
        details->setMinimumWidth(320);
        auto *detailsLayout = new QVBoxLayout(details);
        detailsLayout->setContentsMargins(12, 8, 12, 8);
        auto *detailImage = new QLabel;
        detailImage->setMinimumHeight(190);
        detailImage->setAlignment(Qt::AlignCenter);
        auto *detailTitle = new QLabel;
        QFont detailFont = detailTitle->font(); detailFont.setPointSize(detailFont.pointSize() + 4); detailFont.setBold(true); detailTitle->setFont(detailFont); detailTitle->setWordWrap(true);
        auto *layoutChoice = new QComboBox;
        if (!cars) {
            layoutChoice->setIconSize(QSize(144, 81));
            layoutChoice->setMinimumHeight(88);
        }
        auto *detailText = new QLabel;
        detailText->setWordWrap(true);
        detailText->setTextInteractionFlags(Qt::TextSelectableByMouse);
        detailText->setAlignment(Qt::AlignTop | Qt::AlignLeft);
        auto *favorite = new QPushButton(QIcon::fromTheme("rating"), tr("Add Favorite"));
        auto *dashboard = new QPushButton(QIcon::fromTheme("view-grid"), tr("Add to Dashboard"));
        auto *use = new QPushButton(QIcon::fromTheme("dialog-ok-apply"), cars ? tr("Use This Car") : tr("Use This Track"));
        use->setEnabled(false); favorite->setEnabled(false); dashboard->setEnabled(false);
        detailsLayout->addWidget(detailImage);
        detailsLayout->addWidget(detailTitle);
        detailsLayout->addWidget(layoutChoice);
        detailsLayout->addWidget(detailText, 1);
        detailsLayout->addWidget(favorite);
        detailsLayout->addWidget(dashboard);
        detailsLayout->addWidget(use);
        auto *splitter = new QSplitter;
        splitter->addWidget(list); splitter->addWidget(details); splitter->setStretchFactor(0, 1);
        layout->addLayout(header);
        layout->addWidget(splitter, 1);
        stack_->addWidget(page);
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        catalog = {list, search, filter, sort, favoritesOnly, detailImage, detailTitle, detailText, layoutChoice, favorite, dashboard, use, {}, cars};
        if (cars) { carSearch_ = search; carList_ = list; carCountLabel_ = count; }
        else { trackSearch_ = search; trackList_ = list; trackCountLabel_ = count; }
        auto *searchTimer = new QTimer(search);
        searchTimer->setSingleShot(true);
        searchTimer->setInterval(140);
        connect(search, &QLineEdit::textChanged, searchTimer, qOverload<>(&QTimer::start));
        connect(searchTimer, &QTimer::timeout, this, [this, cars] { filterCatalogSearch(cars ? carList_ : trackList_); });
        connect(filter, &QComboBox::currentIndexChanged, this, [this, cars] { populateCatalog(cars ? carList_ : trackList_); });
        connect(sort, &QComboBox::currentIndexChanged, this, [this, cars] { populateCatalog(cars ? carList_ : trackList_); });
        connect(favoritesOnly, &QCheckBox::toggled, this, [this, cars] { populateCatalog(cars ? carList_ : trackList_); });
        connect(list, &QListWidget::itemClicked, this, [this, cars](QListWidgetItem *item) { inspectCatalogItem(cars, item->data(Qt::UserRole).toJsonObject(), QString()); });
        connect(list, &QListWidget::itemDoubleClicked, this, [this, cars](QListWidgetItem *item) { inspectCatalogItem(cars, item->data(Qt::UserRole).toJsonObject(), QString()); commitCatalogItem(cars); });
        connect(layoutChoice, &QComboBox::currentIndexChanged, this, [this, cars](int index) { if (index >= 0) showCatalogDetails(cars); });
        connect(use, &QPushButton::clicked, this, [this, cars] { commitCatalogItem(cars); });
        connect(favorite, &QPushButton::clicked, this, [this, cars] { toggleCatalogFavorite(cars); });
        connect(dashboard, &QPushButton::clicked, this, [this, cars] { toggleCatalogDashboard(cars); });
    }

    void buildRace()
    {
        auto *content = new QWidget;
        auto *layout = new QVBoxLayout(content);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *label = new QLabel(tr("Race Setup"));
        QFont font = label->font(); font.setPointSize(font.pointSize() + 6); font.setBold(true); label->setFont(font);
        layout->addWidget(label);
        auto *body = new QSplitter;
        auto *tabs = new QTabWidget;
        auto *session = new QWidget; auto *sessionForm = new QFormLayout(session);
        sessionForm->addRow(tr("Session mode"), mirrorCombo(mode_));
        sessionForm->addRow(tr("Opponents"), mirrorSpin(opponents_));
        sessionForm->addRow(tr("AI strength"), mirrorSpin(ai_));
        sessionForm->addRow(tr("Race laps"), mirrorSpin(laps_));
        sessionForm->addRow(tr("Duration"), mirrorSpin(duration_));
        tabs->addTab(session, tr("Session"));
        auto *weatherTab = new QWidget; auto *weatherForm = new QFormLayout(weatherTab);
        const QList<QPair<QString, QString>> advanced = {
            {"time_multiplier", tr("Time multiplier")}, {"cloud_speed", tr("Cloud speed")},
            {"wind_speed_min", tr("Minimum wind speed")}, {"wind_speed_max", tr("Maximum wind speed")},
            {"wind_direction", tr("Wind direction")}, {"session_start_grip", tr("Starting grip")},
            {"session_transfer", tr("Session transfer")}, {"randomness", tr("Grip randomness")},
            {"lap_gain", tr("Laps per grip step")}, {"virtual_laps", tr("Virtual laps")},
            {"max_laps", tr("Maximum groove laps")}, {"starting_laps", tr("Starting groove laps")}};
        for (const auto &[key, labelText] : advanced) {
            auto *box = new QDoubleSpinBox;
            if (key == "time_multiplier") { box->setRange(0, 60); box->setDecimals(1); }
            else if (key == "cloud_speed") { box->setRange(0, 10); box->setDecimals(2); }
            else if (key == "wind_direction") box->setRange(0, 359);
            else if (key == "lap_gain") box->setRange(1, 1000);
            else box->setRange(0, 100);
            box->setKeyboardTracking(false);
            conditionEditors_.insert(key, box);
            connect(box, &QDoubleSpinBox::valueChanged, this, [this, key](double value) {
                if (updating_) return;
                conditions_[key] = value;
                scheduleApply();
            });
            weatherForm->addRow(labelText, sliderControl(box));
        }
        tabs->addTab(weatherTab, tr("Weather & Track"));
        body->addWidget(tabs);
        auto *presets = new QGroupBox(tr("Session Presets"));
        presets->setMinimumWidth(270);
        auto *presetLayout = new QVBoxLayout(presets);
        presetName_ = new QLineEdit; presetName_->setPlaceholderText(tr("Preset name"));
        presetList_ = new QComboBox;
        auto *save = new QPushButton(tr("Save")); auto *load = new QPushButton(tr("Load")); auto *remove = new QPushButton(tr("Delete"));
        connect(save, &QPushButton::clicked, this, [this] { const QString json = QString::fromUtf8(QJsonDocument(conditions_).toJson(QJsonDocument::Compact)); QMetaObject::invokeMethod(backend_, "savePreset", Q_ARG(QString, presetName_->text()), Q_ARG(QString, selectedCar_), Q_ARG(QString, selectedSkin_), Q_ARG(QString, selectedTrack_), Q_ARG(QString, json)); });
        connect(load, &QPushButton::clicked, this, [this] { QMetaObject::invokeMethod(backend_, "applyPreset", Q_ARG(QString, presetList_->currentData().toString())); });
        connect(remove, &QPushButton::clicked, this, [this] { QMetaObject::invokeMethod(backend_, "deletePreset", Q_ARG(QString, presetList_->currentData().toString())); });
        auto *saveRow = new QHBoxLayout; saveRow->addWidget(presetName_, 1); saveRow->addWidget(save);
        auto *presetButtons = new QHBoxLayout; presetButtons->addWidget(load); presetButtons->addWidget(remove);
        presetLayout->addLayout(saveRow); presetLayout->addWidget(presetList_); presetLayout->addLayout(presetButtons); presetLayout->addStretch();
        body->addWidget(presets); body->setStretchFactor(0, 1);
        layout->addWidget(body, 1);
        stack_->addWidget(content);
    }

    QComboBox *mirrorCombo(QComboBox *source)
    {
        auto *combo = new QComboBox;
        for (int i = 0; i < source->count(); ++i) combo->addItem(source->itemText(i), source->itemData(i));
        connect(combo, &QComboBox::activated, this, [this, source, combo](int index) { source->setCurrentIndex(index); scheduleApply(); });
        connect(&poll_, &QTimer::timeout, combo, [source, combo] { if (!combo->hasFocus()) combo->setCurrentIndex(source->currentIndex()); });
        return combo;
    }

    QWidget *mirrorSpin(QSpinBox *source)
    {
        auto *box = spin(source->minimum(), source->maximum());
        connect(box, &QSpinBox::valueChanged, this, [this, source](int value) { source->setValue(value); scheduleApply(); });
        connect(&poll_, &QTimer::timeout, box, [source, box] { if (!box->hasFocus()) box->setValue(source->value()); });
        return source->maximum() - source->minimum() <= 100 ? sliderControl(box) : box;
    }

    void buildOnline()
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *header = new QHBoxLayout;
        auto *title = new QLabel(tr("Online")); QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font);
        onlineSearch_ = new QLineEdit; onlineSearch_->setPlaceholderText(tr("Search servers, tracks, cars, or countries")); onlineSearch_->setClearButtonEnabled(true); onlineSearch_->setMaximumWidth(460);
        auto *refresh = new QPushButton(QIcon::fromTheme("view-refresh"), tr("Refresh Lobby")); connect(refresh, &QPushButton::clicked, this, [this] { invoke(backend_, "refreshOnlineServers"); });
        header->addWidget(title); header->addStretch(); header->addWidget(onlineSearch_); header->addWidget(refresh); layout->addLayout(header);
        onlineTable_ = new QTableWidget; onlineTable_->setColumnCount(7); onlineTable_->setHorizontalHeaderLabels({tr("Server"), tr("Players"), tr("Track"), tr("Cars"), tr("Session"), tr("Country"), tr("Access")}); onlineTable_->verticalHeader()->hide(); onlineTable_->setSelectionBehavior(QAbstractItemView::SelectRows); onlineTable_->setSelectionMode(QAbstractItemView::SingleSelection); onlineTable_->setEditTriggers(QAbstractItemView::NoEditTriggers); onlineTable_->setAlternatingRowColors(true); onlineTable_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch); for (int column = 1; column < 7; ++column) onlineTable_->horizontalHeader()->setSectionResizeMode(column, QHeaderView::ResizeToContents); layout->addWidget(onlineTable_, 1);
        auto *join = new QGroupBox(tr("Join Selected Server")); auto *joinLayout = new QHBoxLayout(join);
        onlineDetails_ = new QLabel(tr("Select a server")); onlineDetails_->setWordWrap(true); joinLayout->addWidget(onlineDetails_, 1);
        onlineCar_ = new QComboBox; onlineCar_->setMinimumWidth(220); onlinePassword_ = new QLineEdit; onlinePassword_->setPlaceholderText(tr("Password")); onlinePassword_->setEchoMode(QLineEdit::Password); onlineJoin_ = new QPushButton(QIcon::fromTheme("network-connect"), tr("Join")); onlineJoin_->setEnabled(false);
        joinLayout->addWidget(new QLabel(tr("Car"))); joinLayout->addWidget(onlineCar_); joinLayout->addWidget(onlinePassword_); joinLayout->addWidget(onlineJoin_); layout->addWidget(join);
        onlineError_ = new QLabel; onlineError_->setWordWrap(true); onlineError_->setForegroundRole(QPalette::Link); layout->addWidget(onlineError_);
        connect(onlineSearch_, &QLineEdit::textChanged, this, [this](const QString &needle) { for (int row = 0; row < onlineTable_->rowCount(); ++row) { bool match = needle.isEmpty(); for (int column = 0; column < onlineTable_->columnCount() && !match; ++column) match = onlineTable_->item(row, column) && onlineTable_->item(row, column)->text().contains(needle, Qt::CaseInsensitive); onlineTable_->setRowHidden(row, !match); } });
        connect(onlineTable_, &QTableWidget::currentCellChanged, this, [this](int row, int, int, int) { selectOnlineServer(row); });
        connect(onlineJoin_, &QPushButton::clicked, this, [this] { const int row = onlineTable_->currentRow(); if (row < 0) return; const int index = onlineTable_->item(row, 0)->data(Qt::UserRole).toInt(); QMetaObject::invokeMethod(backend_, "joinOnlineServer", Q_ARG(int, index), Q_ARG(QString, onlinePassword_->text()), Q_ARG(QString, onlineCar_->currentData().toString())); });
        stack_->addWidget(page);
    }

    void selectOnlineServer(int row)
    {
        if (row < 0 || row >= onlineServers_.size()) { onlineJoin_->setEnabled(false); return; }
        const int index = onlineTable_->item(row, 0)->data(Qt::UserRole).toInt();
        if (index < 0 || index >= onlineServers_.size()) return;
        const QJsonObject server = onlineServers_.at(index).toObject();
        onlineDetails_->setText(tr("%1:%2 | %3/%4 players | %5").arg(server.value("ip").toString()).arg(server.value("tcp_port").toInt()).arg(server.value("clients").toInt()).arg(server.value("capacity").toInt()).arg(server.value("pickup").toBool() ? tr("Pickup enabled") : tr("Booking required")));
        onlineCar_->clear();
        for (const auto &value : server.value("cars").toArray()) { const QString id = value.toString(); onlineCar_->addItem(id, id); }
        const int selected = onlineCar_->findData(selectedCar_); if (selected >= 0) onlineCar_->setCurrentIndex(selected);
        onlinePassword_->setVisible(server.value("passworded").toBool());
        onlineJoin_->setEnabled(onlineCar_->count() > 0);
    }

    void buildReplays()
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *title = new QLabel(tr("Replays"));
        QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font);
        auto *subtitle = new QLabel(tr("Recorded Assetto Corsa sessions")); subtitle->setForegroundRole(QPalette::PlaceholderText);
        layout->addWidget(title); layout->addWidget(subtitle);
        replayList_ = new QListWidget;
        replayList_->setAlternatingRowColors(true);
        layout->addWidget(replayList_, 1);
        stack_->addWidget(page);
    }

    void buildContentManager()
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *header = new QHBoxLayout;
        auto *title = new QLabel(tr("Content Manager"));
        QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font);
        auto *official = new QPushButton(QIcon::fromTheme("internet-web-browser"), tr("Official CSP Releases"));
        connect(official, &QPushButton::clicked, this, [] { QDesktopServices::openUrl(QUrl(QStringLiteral("https://acstuff.club/patch/"))); });
        header->addWidget(title); header->addStretch(); header->addWidget(official); layout->addLayout(header);

        auto *summary = new QGroupBox(tr("Custom Shaders Patch"));
        auto *summaryLayout = new QHBoxLayout(summary);
        contentCspStatus_ = new QLabel; contentCspStatus_->setTextInteractionFlags(Qt::TextSelectableByMouse);
        summaryLayout->addWidget(contentCspStatus_, 1);
        layout->addWidget(summary);

        auto *versions = new QGroupBox(tr("Official Public CSP Versions"));
        auto *versionsLayout = new QVBoxLayout(versions);
        cspReleaseList_ = new QTableWidget; cspReleaseList_->setColumnCount(3); cspReleaseList_->setHorizontalHeaderLabels({tr("Version"), tr("Status"), tr("Download size")}); cspReleaseList_->verticalHeader()->hide(); cspReleaseList_->setSelectionBehavior(QAbstractItemView::SelectRows); cspReleaseList_->setSelectionMode(QAbstractItemView::SingleSelection); cspReleaseList_->setEditTriggers(QAbstractItemView::NoEditTriggers); cspReleaseList_->setMaximumHeight(180); cspReleaseList_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch); cspReleaseList_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch); cspReleaseList_->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents); versionsLayout->addWidget(cspReleaseList_);
        auto *versionActions = new QHBoxLayout; auto *refreshVersions = new QPushButton(QIcon::fromTheme("view-refresh"), tr("Refresh Versions")); cspInstallRelease_ = new QPushButton(QIcon::fromTheme("download"), tr("Download & Install Selected")); cspInstallRelease_->setEnabled(false); versionActions->addWidget(refreshVersions); versionActions->addStretch(); versionActions->addWidget(cspInstallRelease_); versionsLayout->addLayout(versionActions);
        connect(refreshVersions, &QPushButton::clicked, this, [this] { invoke(backend_, "refreshCspReleases"); });
        connect(cspReleaseList_, &QTableWidget::currentCellChanged, this, [this](int row, int, int, int) { cspInstallRelease_->setEnabled(row >= 0 && !boolProperty(backend_, "content_busy")); });
        connect(cspInstallRelease_, &QPushButton::clicked, this, [this] { const int row = cspReleaseList_->currentRow(); if (row < 0) return; const QString version = cspReleaseList_->item(row, 0)->data(Qt::UserRole).toString(); if (QMessageBox::question(this, tr("Install CSP version"), tr("Download CSP v%1 from the official acstuff.club server and install it with a rollback backup?").arg(version)) != QMessageBox::Yes) return; QMetaObject::invokeMethod(backend_, "installCspRelease", Q_ARG(QString, version)); });
        layout->addWidget(versions);

        auto *install = new QGroupBox(tr("Install Local Archive"));
        auto *installLayout = new QVBoxLayout(install);
        auto *chooseRow = new QHBoxLayout;
        contentArchivePath_ = new QLineEdit; contentArchivePath_->setReadOnly(true); contentArchivePath_->setPlaceholderText(tr("Choose a car, track, or CSP ZIP archive"));
        auto *choose = new QPushButton(QIcon::fromTheme("document-open"), tr("Choose ZIP..."));
        connect(choose, &QPushButton::clicked, this, [this] {
            const QString path = QFileDialog::getOpenFileName(this, tr("Choose Assetto Corsa content archive"), QString(), tr("ZIP archives (*.zip)"));
            if (path.isEmpty()) return;
            contentArchivePath_->setText(path);
            QMetaObject::invokeMethod(backend_, "inspectContentArchive", Q_ARG(QString, path));
        });
        chooseRow->addWidget(contentArchivePath_, 1); chooseRow->addWidget(choose); installLayout->addLayout(chooseRow);
        contentPreview_ = new QLabel(tr("No archive selected")); contentPreview_->setWordWrap(true); contentPreview_->setMinimumHeight(54); installLayout->addWidget(contentPreview_);
        auto *installRow = new QHBoxLayout; installRow->addStretch();
        contentInstall_ = new QPushButton(QIcon::fromTheme("archive-insert"), tr("Install with Backup")); contentInstall_->setEnabled(false);
        connect(contentInstall_, &QPushButton::clicked, this, [this] {
            if (contentReplacements_ > 0 && QMessageBox::question(this, tr("Replace existing files"), tr("This archive will replace %1 existing files. Backups will be kept for rollback. Continue?").arg(contentReplacements_)) != QMessageBox::Yes) return;
            invoke(backend_, "installInspectedArchive");
        });
        installRow->addWidget(contentInstall_); installLayout->addLayout(installRow); layout->addWidget(install);

        auto *history = new QGroupBox(tr("Installation History"));
        auto *historyLayout = new QVBoxLayout(history);
        contentHistory_ = new QListWidget; contentHistory_->setAlternatingRowColors(true); historyLayout->addWidget(contentHistory_, 1);
        auto *rollbackRow = new QHBoxLayout; rollbackRow->addStretch(); contentRollback_ = new QPushButton(QIcon::fromTheme("edit-undo"), tr("Roll Back Selected")); contentRollback_->setEnabled(false); rollbackRow->addWidget(contentRollback_); historyLayout->addLayout(rollbackRow);
        connect(contentHistory_, &QListWidget::currentItemChanged, this, [this](QListWidgetItem *current) { contentRollback_->setEnabled(current && !boolProperty(backend_, "content_busy")); });
        connect(contentRollback_, &QPushButton::clicked, this, [this] {
            auto *item = contentHistory_->currentItem(); if (!item) return;
            if (QMessageBox::question(this, tr("Roll back content"), tr("Restore files from the selected installation backup?")) != QMessageBox::Yes) return;
            const QString id = item->data(Qt::UserRole).toString();
            QMetaObject::invokeMethod(backend_, "rollbackContentInstall", Q_ARG(QString, id));
        });
        layout->addWidget(history, 1);
        contentError_ = new QLabel; contentError_->setWordWrap(true); contentError_->setForegroundRole(QPalette::Link); layout->addWidget(contentError_);
        const int pageIndex = stack_->addWidget(page);
        connect(navigation_, &QListWidget::currentRowChanged, this, [this, pageIndex](int row) {
            if (row == pageIndex && !cspReleaseRequested_) {
                cspReleaseRequested_ = true;
                invoke(backend_, "refreshCspReleases");
            }
        });
    }

    void buildTools()
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(12);
        auto *title = new QLabel(tr("Showroom & Setups")); QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font); layout->addWidget(title);
        auto *showroom = new QGroupBox(tr("Showroom")); auto *showroomLayout = new QHBoxLayout(showroom);
        showroomSelection_ = new QLabel; showroomSelection_->setWordWrap(true); showroomLayout->addWidget(showroomSelection_, 1);
        auto *launch = new QPushButton(QIcon::fromTheme("applications-graphics"), tr("Open Current Car in CSP Showroom")); launch->setMinimumHeight(42); connect(launch, &QPushButton::clicked, this, [this] { invoke(backend_, "launchShowroom"); }); showroomLayout->addWidget(launch); layout->addWidget(showroom);
        auto *setups = new QGroupBox(tr("Car Setups")); auto *setupsLayout = new QVBoxLayout(setups);
        setupSummary_ = new QLabel; setupSummary_->setForegroundRole(QPalette::PlaceholderText); setupsLayout->addWidget(setupSummary_);
        setupList_ = new QTableWidget; setupList_->setColumnCount(3); setupList_->setHorizontalHeaderLabels({tr("Name"), tr("Track"), tr("Modified")}); setupList_->verticalHeader()->hide(); setupList_->setSelectionBehavior(QAbstractItemView::SelectRows); setupList_->setSelectionMode(QAbstractItemView::SingleSelection); setupList_->setEditTriggers(QAbstractItemView::NoEditTriggers); setupList_->setAlternatingRowColors(true); setupList_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch); setupList_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch); setupList_->horizontalHeader()->setSectionResizeMode(2, QHeaderView::ResizeToContents); setupsLayout->addWidget(setupList_, 1);
        auto *actions = new QHBoxLayout; setupTrack_ = new QComboBox; setupTrack_->setEditable(true); setupTrack_->addItem(QStringLiteral("generic")); setupName_ = new QLineEdit; setupName_->setPlaceholderText(tr("New setup name")); auto *save = new QPushButton(tr("Save Current As")); setupUse_ = new QPushButton(tr("Use Selected")); setupDelete_ = new QPushButton(tr("Delete Selected")); setupUse_->setEnabled(false); setupDelete_->setEnabled(false);
        actions->addWidget(new QLabel(tr("Track"))); actions->addWidget(setupTrack_); actions->addWidget(setupName_, 1); actions->addWidget(save); actions->addWidget(setupUse_); actions->addWidget(setupDelete_); setupsLayout->addLayout(actions);
        connect(save, &QPushButton::clicked, this, [this] { QMetaObject::invokeMethod(backend_, "saveCurrentSetupAs", Q_ARG(QString, setupTrack_->currentText()), Q_ARG(QString, setupName_->text())); });
        connect(setupList_, &QTableWidget::currentCellChanged, this, [this](int row, int, int, int) { const bool selected = row >= 0; setupUse_->setEnabled(selected); setupDelete_->setEnabled(selected && setupList_->item(row, 0)->text() != "last"); });
        connect(setupUse_, &QPushButton::clicked, this, [this] { const int row = setupList_->currentRow(); if (row < 0) return; const QString id = setupList_->item(row, 0)->data(Qt::UserRole).toString(); QMetaObject::invokeMethod(backend_, "applySetup", Q_ARG(QString, id)); });
        connect(setupDelete_, &QPushButton::clicked, this, [this] { const int row = setupList_->currentRow(); if (row < 0 || QMessageBox::question(this, tr("Delete setup"), tr("Delete the selected car setup?")) != QMessageBox::Yes) return; const QString id = setupList_->item(row, 0)->data(Qt::UserRole).toString(); QMetaObject::invokeMethod(backend_, "deleteSetup", Q_ARG(QString, id)); });
        layout->addWidget(setups, 1);
        toolsError_ = new QLabel; toolsError_->setWordWrap(true); toolsError_->setForegroundRole(QPalette::Link); layout->addWidget(toolsError_);
        stack_->addWidget(page);
    }

    void buildSettings()
    {
        auto *page = new QWidget;
        auto *layout = new QVBoxLayout(page);
        layout->setContentsMargins(20, 16, 20, 20);
        layout->setSpacing(10);
        auto *header = new QHBoxLayout;
        auto *title = new QLabel(tr("Settings")); QFont font = title->font(); font.setPointSize(font.pointSize() + 7); font.setBold(true); title->setFont(font);
        settingsSearch_ = new QLineEdit;
        settingsSearch_->setPlaceholderText(tr("Search settings, sections and options"));
        settingsSearch_->setClearButtonEnabled(true);
        settingsSearch_->setMaximumWidth(460);
        header->addWidget(title); header->addStretch(); header->addWidget(settingsSearch_);
        settingsKind_ = new QTabBar;
        settingsKind_->setExpanding(false);
        settingsKind_->addTab(tr("Assetto Corsa"));
        settingsKind_->addTab(tr("Custom Shaders Patch"));
        auto *splitter = new QSplitter;
        moduleList_ = new QListWidget;
        moduleList_->setMinimumWidth(210);
        moduleList_->setMaximumWidth(270);
        settingsScroll_ = new QScrollArea;
        settingsScroll_->setWidgetResizable(true);
        settingsScroll_->setFrameShape(QFrame::NoFrame);
        settingsScroll_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
        splitter->addWidget(moduleList_); splitter->addWidget(settingsScroll_); splitter->setStretchFactor(1, 1);
        layout->addLayout(header); layout->addWidget(settingsKind_); layout->addWidget(splitter, 1);
        stack_->addWidget(page);
        connect(settingsKind_, &QTabBar::currentChanged, this, [this] { populateModules(); });
        connect(settingsSearch_, &QLineEdit::textChanged, this, [this] { populateModules(); });
        connect(moduleList_, &QListWidget::currentRowChanged, this, [this](int row) { showModule(row); });
        auto *find = new QShortcut(QKeySequence::Find, this);
        connect(find, &QShortcut::activated, this, [this] { navigation_->setCurrentRow(8); settingsSearch_->setFocus(); settingsSearch_->selectAll(); });
    }

    void populateModules()
    {
        const QJsonArray source = settingsKind_->currentIndex() == 0 ? acModules_ : cspModules_;
        const QString needle = settingsSearch_->text();
        moduleList_->clear(); visibleModules_ = {};
        for (const QJsonValue &value : source) {
            const QJsonObject module = value.toObject();
            const QByteArray searchable = QJsonDocument(module).toJson(QJsonDocument::Compact);
            if (!needle.isEmpty() && !QString::fromUtf8(searchable).contains(needle, Qt::CaseInsensitive)) continue;
            visibleModules_.append(module);
            moduleList_->addItem(module.value("name").toString());
        }
        if (moduleList_->count()) moduleList_->setCurrentRow(0);
    }

    void showModule(int row)
    {
        if (row < 0 || row >= visibleModules_.size()) return;
        const QJsonObject module = visibleModules_.at(row).toObject();
        axisMeters_.clear();
        buttonIndicators_.clear();
        if (module.value("file").toString() == "controls.ini") { showControls(); return; }
        if (module.value("file").toString() == "acos.ini") { showAppLayout(); return; }
        QMetaObject::invokeMethod(backend_, "setControlMonitoring", Q_ARG(bool, false));
        auto *content = new QWidget; auto *layout = new QVBoxLayout(content);
        auto *heading = new QLabel(module.value("name").toString()); QFont font = heading->font(); font.setPointSize(font.pointSize() + 5); font.setBold(true); heading->setFont(font); layout->addWidget(heading);
        for (const QJsonValue &sectionValue : module.value("sections").toArray()) {
            const QJsonObject section = sectionValue.toObject();
            auto *group = new QGroupBox(section.value("name").toString()); auto *form = new QFormLayout(group); form->setRowWrapPolicy(QFormLayout::WrapLongRows); form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
            for (const QJsonValue &optionValue : section.value("options").toArray()) {
                const QJsonObject option = optionValue.toObject();
                const QString label = option.value("label").toString(); const QString key = option.value("key").toString();
                const QString sectionId = section.value("id").toString(); const QString file = module.value("file").toString();
                if (option.value("kind").toString() == "bool") {
                    auto *check = new QCheckBox; check->setChecked(option.value("value").toString() == "1");
                    connect(check, &QCheckBox::toggled, this, [this, file, sectionId, key](bool checked) { saveSetting(file, sectionId, key, checked ? "1" : "0"); });
                    form->addRow(label, check);
                } else if (option.value("kind").toString() == "slider") {
                    auto *box = new QDoubleSpinBox; box->setRange(option.value("minimum").toDouble(), option.value("maximum").toDouble()); box->setSingleStep(option.value("step").toDouble()); box->setDecimals(option.value("decimals").toInt()); box->setValue(option.value("value").toString().toDouble()); box->setSuffix(option.value("unit").toString());
                    connect(box, &QDoubleSpinBox::editingFinished, this, [this, file, sectionId, key, box] { saveSetting(file, sectionId, key, QString::number(box->value(), 'f', box->decimals())); });
                    form->addRow(label, sliderControl(box));
                } else {
                    auto *edit = new QLineEdit(option.value("value").toString());
                    connect(edit, &QLineEdit::editingFinished, this, [this, file, sectionId, key, edit] { saveSetting(file, sectionId, key, edit->text()); });
                    form->addRow(label, edit);
                }
            }
            layout->addWidget(group);
        }
        content->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred); layout->addStretch(); settingsScroll_->setWidget(content);
    }

    void showControls()
    {
        QMetaObject::invokeMethod(backend_, "setControlMonitoring", Q_ARG(bool, true));
        const QJsonObject controls = controls_;
        auto *content = new QWidget; auto *layout = new QVBoxLayout(content); layout->setContentsMargins(12, 8, 12, 12); layout->setSpacing(10);
        auto *heading = new QLabel(tr("Controls")); QFont font = heading->font(); font.setPointSize(font.pointSize() + 6); font.setBold(true); heading->setFont(font); layout->addWidget(heading);
        auto *controller = new QGroupBox(tr("Controller Profile")); auto *controllerLayout = new QGridLayout(controller);
        auto *method = new QComboBox; method->addItem(tr("Wheel"), "WHEEL"); method->addItem(tr("Gamepad"), "X360"); method->addItem(tr("Keyboard"), "KEYBOARD"); method->setCurrentIndex(method->findData(controls.value("input_method").toString()));
        connect(method, &QComboBox::activated, this, [this, method] { invokeControlOption("HEADER", "INPUT_METHOD", method->currentData().toString()); });
        auto *preset = new QComboBox; for (const auto &value : controls.value("presets").toArray()) { const auto item = value.toObject(); preset->addItem(item.value("name").toString(), item.value("name").toString()); }
        auto *loadPreset = new QPushButton(tr("Load Preset")); connect(loadPreset, &QPushButton::clicked, this, [this, preset] { QMetaObject::invokeMethod(backend_, "loadControlPreset", Q_ARG(QString, preset->currentData().toString())); });
        auto *presetName = new QLineEdit; presetName->setPlaceholderText(tr("New preset name")); auto *savePreset = new QPushButton(tr("Save Preset")); connect(savePreset, &QPushButton::clicked, this, [this, presetName] { QMetaObject::invokeMethod(backend_, "saveControlPreset", Q_ARG(QString, presetName->text())); });
        controllerLayout->addWidget(new QLabel(tr("Input method")), 0, 0); controllerLayout->addWidget(method, 0, 1);
        controllerLayout->addWidget(new QLabel(tr("Saved preset")), 0, 2); controllerLayout->addWidget(preset, 0, 3); controllerLayout->addWidget(loadPreset, 0, 4);
        controllerLayout->addWidget(new QLabel(tr("Save current setup")), 1, 2); controllerLayout->addWidget(presetName, 1, 3); controllerLayout->addWidget(savePreset, 1, 4); controllerLayout->setColumnStretch(3, 1);
        layout->addWidget(controller);
        QStringList connectedDevices; for (const QJsonValue &value : controls.value("devices").toArray()) { const auto device = value.toObject(); if (device.value("connected").toBool()) connectedDevices.append(device.value("name").toString()); }
        auto *devices = new QLabel(connectedDevices.isEmpty() ? tr("No connected controllers detected") : tr("Connected: %1").arg(connectedDevices.join(QStringLiteral("  |  ")))); devices->setWordWrap(true); devices->setForegroundRole(QPalette::PlaceholderText); layout->addWidget(devices);
        auto *tabs = new QTabWidget;
        auto *axes = new QTableWidget; axes->setColumnCount(6); axes->setHorizontalHeaderLabels({tr("Function"), tr("Assignment"), tr("Live input"), tr("Invert"), tr("Assign"), tr("Clear")}); axes->verticalHeader()->hide(); axes->verticalHeader()->setDefaultSectionSize(38); axes->setAlternatingRowColors(true); axes->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents); axes->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch); axes->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Fixed); axes->horizontalHeader()->resizeSection(2, 110); axes->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Fixed); axes->horizontalHeader()->resizeSection(3, 62); axes->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Fixed); axes->horizontalHeader()->resizeSection(4, 86); axes->horizontalHeader()->setSectionResizeMode(5, QHeaderView::Fixed); axes->horizontalHeader()->resizeSection(5, 66);
        const QJsonArray axisData = controls.value("axes").toArray(); axes->setRowCount(axisData.size()); axisMeters_.clear();
        for (int row = 0; row < axisData.size(); ++row) { const auto axis = axisData.at(row).toObject(); axes->setItem(row, 0, new QTableWidgetItem(axis.value("label").toString())); axes->setItem(row, 1, new QTableWidgetItem(axis.value("assignment").toString())); auto *meter = new QProgressBar; meter->setRange(0, 100); meter->setValue(0); meter->setTextVisible(false); axes->setCellWidget(row, 2, meter); axisMeters_.append({meter, axis.value("joy").toInt(-1), axis.value("axis").toInt(-1)}); auto *invert = new QCheckBox; invert->setChecked(axis.value("inverted").toBool()); invert->setEnabled(axis.value("section").toString() != "STEER"); axes->setCellWidget(row, 3, invert); const QString section = axis.value("section").toString(); auto *assign = new QPushButton(tr("Assign...")); connect(assign, &QPushButton::clicked, this, [this, section, axis] { capture(section, "AXLE", axis.value("label").toString()); }); axes->setCellWidget(row, 4, assign); auto *clear = new QPushButton(tr("Clear")); connect(clear, &QPushButton::clicked, this, [this, section] { QMetaObject::invokeMethod(backend_, "clearControlBinding", Q_ARG(QString, section), Q_ARG(QString, QStringLiteral("AXLE"))); }); axes->setCellWidget(row, 5, clear); }
        tabs->addTab(axes, tr("Driving Axes"));
        auto *bindings = new QTableWidget; bindings->setColumnCount(6); bindings->setHorizontalHeaderLabels({tr("Function"), tr("Category"), tr("Assignment"), tr("Live"), tr("Assign"), tr("Clear")}); bindings->verticalHeader()->hide(); bindings->verticalHeader()->setDefaultSectionSize(38); bindings->setAlternatingRowColors(true); bindings->horizontalHeader()->setSectionResizeMode(0, QHeaderView::ResizeToContents); bindings->horizontalHeader()->setSectionResizeMode(1, QHeaderView::ResizeToContents); bindings->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch); bindings->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Fixed); bindings->horizontalHeader()->resizeSection(3, 72); bindings->horizontalHeader()->setSectionResizeMode(4, QHeaderView::Fixed); bindings->horizontalHeader()->resizeSection(4, 86); bindings->horizontalHeader()->setSectionResizeMode(5, QHeaderView::Fixed); bindings->horizontalHeader()->resizeSection(5, 66); const QJsonArray bindingData = controls.value("bindings").toArray(); bindings->setRowCount(bindingData.size()); buttonIndicators_.clear();
        for (int row = 0; row < bindingData.size(); ++row) { const auto binding = bindingData.at(row).toObject(); const QString section = binding.value("section").toString(); bindings->setItem(row, 0, new QTableWidgetItem(binding.value("label").toString())); bindings->setItem(row, 1, new QTableWidgetItem(binding.value("category").toString())); bindings->setItem(row, 2, new QTableWidgetItem(binding.value("assignment").toString())); auto *live = new QLabel(tr("Released")); live->setAlignment(Qt::AlignCenter); bindings->setCellWidget(row, 3, live); buttonIndicators_.append({live, binding.value("joy").toInt(-1), binding.value("button").toInt(-1)}); auto *assign = new QPushButton(tr("Assign...")); connect(assign, &QPushButton::clicked, this, [this, section, binding] { capture(section, "BUTTON", binding.value("label").toString()); }); bindings->setCellWidget(row, 4, assign); auto *clear = new QPushButton(tr("Clear")); connect(clear, &QPushButton::clicked, this, [this, section] { QMetaObject::invokeMethod(backend_, "clearControlBinding", Q_ARG(QString, section), Q_ARG(QString, QStringLiteral("BUTTON"))); }); bindings->setCellWidget(row, 5, clear); }
        tabs->addTab(bindings, tr("Button Bindings"));
        auto *shifter = new QWidget; auto *shifterForm = new QFormLayout(shifter); const auto shifterData = controls.value("shifter").toObject(); auto *shifterActive = new QCheckBox; shifterActive->setChecked(shifterData.value("active").toBool()); connect(shifterActive, &QCheckBox::toggled, this, [this](bool checked) { invokeControlOption("SHIFTER", "ACTIVE", checked ? "1" : "0"); }); shifterForm->addRow(tr("Use H-pattern shifter"), shifterActive); for (const auto &value : shifterData.value("gears").toArray()) { const auto gear = value.toObject(); auto *row = new QWidget; auto *rowLayout = new QHBoxLayout(row); rowLayout->setContentsMargins(0, 0, 0, 0); rowLayout->addWidget(new QLabel(gear.value("assignment").toString()), 1); auto *assign = new QPushButton(tr("Assign...")); const QString key = gear.value("key").toString(); const QString label = gear.value("label").toString(); connect(assign, &QPushButton::clicked, this, [this, key, label] { capture("SHIFTER", key, label); }); rowLayout->addWidget(assign); shifterForm->addRow(label, row); } tabs->addTab(shifter, tr("H-Pattern Shifter"));
        auto *ffb = new QWidget; auto *ffbForm = new QFormLayout(ffb); const auto steering = controls.value("steering").toObject(); const auto feedback = controls.value("force_feedback").toObject(); for (const QString key : {"FF_GAIN", "FILTER_FF"}) { auto *box = new QDoubleSpinBox; box->setRange(0, 1); box->setSingleStep(0.01); box->setValue(steering.value(key).toString().toDouble()); connect(box, &QDoubleSpinBox::editingFinished, this, [this, key, box] { invokeControlOption("STEER", key, QString::number(box->value(), 'f', 2)); }); ffbForm->addRow(key, box); } for (auto it = feedback.begin(); it != feedback.end(); ++it) { auto *box = new QDoubleSpinBox; box->setRange(0, 1); box->setSingleStep(0.01); box->setValue(it.value().toString().toDouble()); const QString key = it.key(); connect(box, &QDoubleSpinBox::editingFinished, this, [this, key, box] { invokeControlOption("FF_EXPERIMENTAL", key, QString::number(box->value(), 'f', 2)); }); ffbForm->addRow(key, box); } tabs->addTab(ffb, tr("Force Feedback"));
        layout->addWidget(tabs); settingsScroll_->setWidget(content);
    }

    void showAppLayout()
    {
        QMetaObject::invokeMethod(backend_, "setControlMonitoring", Q_ARG(bool, false));
        auto *content = new QWidget; auto *outer = new QVBoxLayout(content);
        auto *desktopRow = new QHBoxLayout; desktopRow->addWidget(new QLabel(tr("Desktop"))); auto *desktop = new QComboBox; for (int i = 1; i <= 4; ++i) desktop->addItem(QString::number(i), i); desktop->setCurrentIndex(appLayout_.value("selected_desktop").toInt(1) - 1); connect(desktop, &QComboBox::activated, this, [this, desktop] { QMetaObject::invokeMethod(backend_, "setAppLayoutOption", Q_ARG(QString, QStringLiteral("HEADER")), Q_ARG(QString, QStringLiteral("DESKTOP_SELECTED")), Q_ARG(QString, desktop->currentData().toString())); }); desktopRow->addWidget(desktop); desktopRow->addStretch(); outer->addLayout(desktopRow);
        auto *layout = new QHBoxLayout; outer->addLayout(layout, 1);
        auto *scene = new QGraphicsScene(content); scene->setSceneRect(0, 0, 960, 540);
        auto *view = new QGraphicsView(scene); view->setRenderHint(QPainter::Antialiasing); view->fitInView(scene->sceneRect(), Qt::KeepAspectRatio);
        auto *list = new QTableWidget; list->setColumnCount(5); list->setHorizontalHeaderLabels({tr("App"), tr("Visible"), tr("Locked"), tr("X"), tr("Y")}); list->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
        const QJsonArray allApps = appLayout_.value("apps").toArray(); QJsonArray apps; const int selectedDesktop = appLayout_.value("selected_desktop").toInt(1); for (const auto &value : allApps) if (value.toObject().value("desktop").toInt() == selectedDesktop) apps.append(value); list->setRowCount(apps.size());
        const qreal screenWidth = qMax(1, appLayout_.value("screen_width").toInt(1920)); const qreal screenHeight = qMax(1, appLayout_.value("screen_height").toInt(1080));
        for (int row = 0; row < apps.size(); ++row) { const auto app = apps.at(row).toObject(); const QString section = app.value("section").toString(); list->setItem(row, 0, new QTableWidgetItem(app.value("name").toString())); auto *visible = new QCheckBox; visible->setChecked(app.value("visible").toBool()); connect(visible, &QCheckBox::toggled, this, [this, section](bool checked) { QMetaObject::invokeMethod(backend_, "setAppLayoutOption", Q_ARG(QString, section), Q_ARG(QString, QStringLiteral("VISIBLE")), Q_ARG(QString, checked ? QStringLiteral("1") : QStringLiteral("0"))); }); list->setCellWidget(row, 1, visible); auto *locked = new QCheckBox; locked->setChecked(app.value("blocked").toBool()); connect(locked, &QCheckBox::toggled, this, [this, section](bool checked) { QMetaObject::invokeMethod(backend_, "setAppLayoutOption", Q_ARG(QString, section), Q_ARG(QString, QStringLiteral("BLOCKED")), Q_ARG(QString, checked ? QStringLiteral("1") : QStringLiteral("0"))); }); list->setCellWidget(row, 2, locked); list->setItem(row, 3, new QTableWidgetItem(QString::number(app.value("x").toInt()))); list->setItem(row, 4, new QTableWidgetItem(QString::number(app.value("y").toInt()))); if (app.value("visible").toBool()) { const qreal x = app.value("x").toDouble() * 960.0 / screenWidth; const qreal y = app.value("y").toDouble() * 540.0 / screenHeight; scene->addRect(x, y, 150, 44, QPen(palette().highlight().color()), QBrush(palette().base())); auto *text = scene->addText(app.value("name").toString()); text->setPos(x + 6, y + 8); } }
        layout->addWidget(view, 2); layout->addWidget(list, 1); settingsScroll_->setWidget(content);
    }

    void capture(const QString &section, const QString &key, const QString &label)
    {
        QMetaObject::invokeMethod(backend_, "captureControl", Q_ARG(QString, section), Q_ARG(QString, key), Q_ARG(QString, label));
        statusBar()->showMessage(tr("Move or press the input for %1; capture times out automatically.").arg(label));
    }

    void invokeControlOption(const QString &section, const QString &key, const QString &value)
    {
        QMetaObject::invokeMethod(backend_, "setControlOption", Q_ARG(QString, section), Q_ARG(QString, key), Q_ARG(QString, value));
    }

    void saveSetting(const QString &file, const QString &section, const QString &key, const QString &value)
    {
        invokeOption(backend_, settingsKind_->currentIndex() == 0 ? "setAcOption" : "setCspOption", file, section, key, value);
    }

    void scheduleApply() { if (!updating_) applyTimer_.start(); }

    void applyDrive()
    {
        if (selectedCar_.isEmpty() || selectedTrack_.isEmpty()) return;
        const QString json = driveConditionsJson();
        QMetaObject::invokeMethod(backend_, "applyConfiguration", Q_ARG(QString, selectedCar_), Q_ARG(QString, selectedSkin_), Q_ARG(QString, selectedTrack_), Q_ARG(QString, json));
    }

    QString driveConditionsJson() const
    {
        QJsonObject conditions = conditions_;
        conditions["session_mode"] = mode_->currentData().toString(); conditions["opponents"] = opponents_->value(); conditions["ai_level"] = ai_->value(); conditions["race_laps"] = laps_->value(); conditions["session_duration"] = duration_->value(); conditions["penalties"] = penalties_->isChecked(); conditions["weather_id"] = weather_->currentData().toString(); conditions["sun_angle"] = (time_->time().hour() + time_->time().minute() / 60.0 - 13.0) * 16.0; conditions["ambient_temperature"] = air_->value(); conditions["road_temperature"] = road_->value();
        return QString::fromUtf8(QJsonDocument(conditions).toJson(QJsonDocument::Compact));
    }

    void launchDrive()
    {
        if (boolProperty(backend_, "race_running")) {
            invoke(backend_, "stopRace");
            return;
        }
        if (selectedCar_.isEmpty() || selectedTrack_.isEmpty()) return;
        applyTimer_.stop();
        const QString json = driveConditionsJson();
        QMetaObject::invokeMethod(backend_, "launchConfiguration", Q_ARG(QString, selectedCar_), Q_ARG(QString, selectedSkin_), Q_ARG(QString, selectedTrack_), Q_ARG(QString, json));
    }

    void refresh()
    {
        const bool contentBusy = boolProperty(backend_, "content_busy");
        const bool onlineBusy = boolProperty(backend_, "online_busy");
        const bool launching = boolProperty(backend_, "launching");
        const bool raceRunning = boolProperty(backend_, "race_running");
        if (driveButton_) {
            driveButton_->setEnabled(!launching);
            driveButton_->setText(raceRunning ? tr("Stop") : tr("Drive"));
            driveButton_->setIcon(QIcon::fromTheme(raceRunning ? "media-playback-stop" : "media-playback-start"));
        }
        if (driveAction_) {
            driveAction_->setEnabled(!launching);
            driveAction_->setText(raceRunning ? tr("Stop") : tr("Drive"));
            driveAction_->setIcon(QIcon::fromTheme(raceRunning ? "media-playback-stop" : "media-playback-start"));
        }
        if (onlineJoin_)
            onlineJoin_->setEnabled(!launching && onlineTable_->currentRow() >= 0 && onlineCar_->count() > 0);
        statusLabel_->setText(textProperty(backend_, "status")); cspLabel_->setText(textProperty(backend_, "csp_status")); busy_->setVisible(boolProperty(backend_, "scanning") || boolProperty(backend_, "launching") || contentBusy || onlineBusy);
        if (contentCspStatus_) contentCspStatus_->setText(textProperty(backend_, "csp_status"));
        if (contentError_) contentError_->setText(textProperty(backend_, "error_message"));
        if (onlineError_) onlineError_->setText(textProperty(backend_, "error_message"));
        if (toolsError_) toolsError_->setText(textProperty(backend_, "error_message"));
        if (showroomSelection_) showroomSelection_->setText(tr("%1\n%2").arg(textProperty(backend_, "car_name"), textProperty(backend_, "skin_name")));
        if (launchSummary_) launchSummary_->setText(tr("%1\n\n%2\n\n%3").arg(textProperty(backend_, "car_name"), textProperty(backend_, "track_name"), textProperty(backend_, "csp_status")));
        if (carCountLabel_) carCountLabel_->setText(tr("%1 installed").arg(backend_->property("car_count").toInt()));
        if (trackCountLabel_) trackCountLabel_->setText(tr("%1 tracks, %2 layouts").arg(groupedTracks().size()).arg(backend_->property("track_count").toInt()));
        if (!applyTimer_.isActive()) { selectedCar_ = textProperty(backend_, "car_id"); selectedSkin_ = textProperty(backend_, "skin_id"); selectedTrack_ = textProperty(backend_, "track_id"); const QString layout = textProperty(backend_, "track_layout_id"); if (!layout.isEmpty()) selectedTrack_ += "/" + layout; }
        const QString carsJson = textProperty(backend_, "cars_json"); if (carsJson != carsCache_) { carsCache_ = carsJson; cars_ = QJsonDocument::fromJson(carsJson.toUtf8()).array(); populateCatalog(carList_); }
        const QString tracksJson = textProperty(backend_, "tracks_json"); if (tracksJson != tracksCache_) { tracksCache_ = tracksJson; tracks_ = QJsonDocument::fromJson(tracksJson.toUtf8()).array(); populateCatalog(trackList_); }
        const QString replayJson = textProperty(backend_, "replays_json"); if (replayJson != replayCache_) { replayCache_ = replayJson; replayList_->clear(); for (const auto &value : QJsonDocument::fromJson(replayJson.toUtf8()).array()) replayList_->addItem(value.toObject().value("name").toString()); }
        const QString conditionsJson = textProperty(backend_, "conditions_json"); if (conditionsJson != conditionsCache_) { conditionsCache_ = conditionsJson; conditions_ = QJsonDocument::fromJson(conditionsJson.toUtf8()).object(); updateDrive(); }
        const QString weatherJson = textProperty(backend_, "weather_json"); if (weatherJson != weatherCache_) { weatherCache_ = weatherJson; const auto array = QJsonDocument::fromJson(weatherJson.toUtf8()).array(); weather_->clear(); for (const auto &value : array) { const auto item = value.toObject(); weather_->addItem(item.value("name").toString(), item.value("id").toString()); } updateDrive(); }
        const QString ac = textProperty(backend_, "ac_settings_json"), csp = textProperty(backend_, "csp_settings_json"), controls = textProperty(backend_, "controls_json"); if (ac != acCache_ || csp != cspCache_) { acCache_ = ac; cspCache_ = csp; acModules_ = QJsonDocument::fromJson(ac.toUtf8()).array(); cspModules_ = QJsonDocument::fromJson(csp.toUtf8()).array(); populateModules(); } if (controls != controlsCache_) { controlsCache_ = controls; controls_ = QJsonDocument::fromJson(controls.toUtf8()).object(); if (navigation_->currentRow() == 8 && moduleList_->currentItem() && moduleList_->currentItem()->text().contains("Controls")) showControls(); }
        const QString controlInput = textProperty(backend_, "control_input_json"); if (controlInput != controlInputCache_) { controlInputCache_ = controlInput; const QJsonArray devices = QJsonDocument::fromJson(controlInput.toUtf8()).object().value("devices").toArray(); for (const AxisMeter &mapping : axisMeters_) { bool found = false; double position = 0.0; for (const auto &value : devices) { const QJsonObject device = value.toObject(); if (device.value("controller_index").toInt(-1) != mapping.controller) continue; const QJsonArray axes = device.value("axes").toArray(); if (mapping.axis >= 0 && mapping.axis < axes.size()) { position = axes.at(mapping.axis).toDouble(); found = true; } break; } mapping.meter->setEnabled(found); mapping.meter->setValue(qBound(0, qRound((position + 1.0) * 50.0), 100)); } for (const ButtonIndicator &mapping : buttonIndicators_) { bool found = false; bool pressed = false; for (const auto &value : devices) { const QJsonObject device = value.toObject(); if (device.value("controller_index").toInt(-1) != mapping.controller) continue; const QJsonArray buttons = device.value("buttons").toArray(); if (mapping.button >= 0 && mapping.button < buttons.size()) { pressed = buttons.at(mapping.button).toBool(); found = true; } break; } mapping.label->setEnabled(found); mapping.label->setText(pressed ? tr("Pressed") : tr("Released")); } }
        const QString servers = textProperty(backend_, "servers_json"); if (servers != serversCache_) { serversCache_ = servers; onlineServers_ = QJsonDocument::fromJson(servers.toUtf8()).array(); onlineTable_->setRowCount(onlineServers_.size()); for (int row = 0; row < onlineServers_.size(); ++row) { const auto server = onlineServers_.at(row).toObject(); const QStringList cars = [&server] { QStringList result; for (const auto &value : server.value("cars").toArray()) result.append(value.toString()); return result; }(); const QStringList values = {server.value("name").toString(), tr("%1/%2").arg(server.value("clients").toInt()).arg(server.value("capacity").toInt()), server.value("track").toString(), tr("%1 cars").arg(cars.size()), server.value("session").toString(), server.value("country").toString(), server.value("passworded").toBool() ? tr("Password") : tr("Open")}; for (int column = 0; column < values.size(); ++column) onlineTable_->setItem(row, column, new QTableWidgetItem(values.at(column))); onlineTable_->item(row, 0)->setData(Qt::UserRole, row); onlineTable_->item(row, 3)->setToolTip(cars.join(QStringLiteral(", "))); } }
        const QString setupJson = textProperty(backend_, "setups_json"); if (setupJson != setupsCache_) { setupsCache_ = setupJson; const auto setupItems = QJsonDocument::fromJson(setupJson.toUtf8()).array(); setupList_->setRowCount(setupItems.size()); QSet<QString> tracks; tracks.insert(QStringLiteral("generic")); for (int row = 0; row < setupItems.size(); ++row) { const auto setup = setupItems.at(row).toObject(); auto *name = new QTableWidgetItem(setup.value("name").toString()); name->setData(Qt::UserRole, setup.value("id").toString()); setupList_->setItem(row, 0, name); setupList_->setItem(row, 1, new QTableWidgetItem(setup.value("track").toString())); setupList_->setItem(row, 2, new QTableWidgetItem(QDateTime::fromSecsSinceEpoch(setup.value("modified").toInteger()).toString(QStringLiteral("yyyy-MM-dd HH:mm")))); tracks.insert(setup.value("track").toString()); } const QString currentTrack = setupTrack_->currentText(); setupTrack_->clear(); setupTrack_->addItems(tracks.values()); setupTrack_->setCurrentText(currentTrack.isEmpty() ? QStringLiteral("generic") : currentTrack); setupSummary_->setText(tr("%1 setups for %2").arg(setupItems.size()).arg(textProperty(backend_, "car_name"))); }
        const QString contentPreview = textProperty(backend_, "content_preview_json"); if (contentPreview != contentPreviewCache_) { contentPreviewCache_ = contentPreview; const auto preview = QJsonDocument::fromJson(contentPreview.toUtf8()).object(); contentReplacements_ = preview.value("replacements").toInt(); const QString kind = preview.value("kind").toString(); if (kind.isEmpty()) { contentPreview_->setText(tr("No archive selected")); contentInstall_->setEnabled(false); } else { const QStringList components = [&preview] { QStringList values; for (const auto &value : preview.value("components").toArray()) values.append(value.toString()); return values; }(); contentPreview_->setText(tr("%1\nType: %2   Components: %3\n%4 files, %5 MiB expanded, %6 existing files replaced").arg(preview.value("file_name").toString(), kind.toUpper(), components.join(QStringLiteral(", "))).arg(preview.value("file_count").toInt()).arg(preview.value("expanded_bytes").toDouble() / 1048576.0, 0, 'f', 1).arg(contentReplacements_)); contentInstall_->setEnabled(!contentBusy); } }
        const QString contentHistory = textProperty(backend_, "content_history_json"); if (contentHistory != contentHistoryCache_) { contentHistoryCache_ = contentHistory; contentHistory_->clear(); for (const auto &value : QJsonDocument::fromJson(contentHistory.toUtf8()).array()) { const auto record = value.toObject(); QStringList components; for (const auto &component : record.value("components").toArray()) components.append(component.toString()); auto *item = new QListWidgetItem(tr("%1  |  %2  |  %3").arg(record.value("archive_name").toString(), record.value("kind").toString().toUpper(), components.join(QStringLiteral(", ")))); item->setData(Qt::UserRole, record.value("id").toString()); contentHistory_->addItem(item); } }
        const QString cspReleases = textProperty(backend_, "csp_releases_json"); if (cspReleases != cspReleasesCache_) { cspReleasesCache_ = cspReleases; const QJsonArray releases = QJsonDocument::fromJson(cspReleases.toUtf8()).array(); cspReleaseList_->setRowCount(releases.size()); const QString installed = textProperty(backend_, "csp_status"); for (int row = 0; row < releases.size(); ++row) { const QJsonObject release = releases.at(row).toObject(); QString version = release.value("version").toString(); if (installed.contains(QStringLiteral("v") + version)) version += tr(" (installed)"); auto *versionItem = new QTableWidgetItem(version); versionItem->setData(Qt::UserRole, release.value("version").toString()); cspReleaseList_->setItem(row, 0, versionItem); cspReleaseList_->setItem(row, 1, new QTableWidgetItem(release.value("status").toString())); cspReleaseList_->setItem(row, 2, new QTableWidgetItem(release.value("size").toString())); } }
        if (contentInstall_) contentInstall_->setEnabled(!contentBusy && !QJsonDocument::fromJson(contentPreviewCache_.toUtf8()).object().value("kind").toString().isEmpty());
        if (contentRollback_) contentRollback_->setEnabled(!contentBusy && contentHistory_->currentItem());
        if (cspInstallRelease_) cspInstallRelease_->setEnabled(!contentBusy && cspReleaseList_->currentRow() >= 0);
        const QString presets = textProperty(backend_, "presets_json"); if (presets != presetsCache_) { presetsCache_ = presets; if (presetList_) { presetList_->clear(); for (const auto &value : QJsonDocument::fromJson(presets.toUtf8()).array()) { const auto item = value.toObject(); presetList_->addItem(item.value("name").toString(), item.value("id").toString()); } } }
        const QString appLayout = textProperty(backend_, "app_layout_json"); if (appLayout != appLayoutCache_) { appLayoutCache_ = appLayout; appLayout_ = QJsonDocument::fromJson(appLayout.toUtf8()).object(); }
        updateCarVariants(); updateSkins(); updateTrackLayouts();
        carPreview_->setText(textProperty(backend_, "car_name") + "\n" + textProperty(backend_, "skin_name")); carPreview_->setIcon(QIcon(pixmapFromUrl(selectedCarPreview(), QSize(320, 145)))); trackPreview_->setText(textProperty(backend_, "track_name") + "\n" + textProperty(backend_, "track_layout")); trackPreview_->setIcon(QIcon(pixmapFromUrl(selectedTrackImage(), QSize(320, 145))));
    }

    QVector<QJsonObject> groupedTracks() const
    {
        QVector<QJsonObject> groups;
        QMap<QString, int> positions;
        for (const auto &value : tracks_) {
            const QJsonObject layout = value.toObject();
            const QString base = layout.value("id").toString().section('/', 0, 0);
            int position = positions.value(base, -1);
            if (position < 0) {
                QJsonObject group = layout;
                group["base_id"] = base;
                group["layouts"] = QJsonArray{};
                groups.append(group);
                position = groups.size() - 1;
                positions.insert(base, position);
            }
            QJsonArray layouts = groups[position].value("layouts").toArray();
            layouts.append(layout);
            groups[position]["layouts"] = layouts;
            groups[position]["favorite"] = groups[position].value("favorite").toBool() || layout.value("favorite").toBool();
            groups[position]["dashboard"] = groups[position].value("dashboard").toBool() || layout.value("dashboard").toBool();
        }
        for (QJsonObject &group : groups) {
            const QString fallback = humanizeTrackId(group.value("base_id").toString());
            group["name"] = sharedTrackName(group.value("layouts").toArray(), fallback);
            if (group.value("preview").toString().isEmpty()) {
                for (const auto &value : group.value("layouts").toArray()) {
                    const QString preview = value.toObject().value("preview").toString();
                    if (!preview.isEmpty()) {
                        group["preview"] = preview;
                        break;
                    }
                }
            }
        }
        return groups;
    }

    QVector<QJsonObject> groupedCars() const
    {
        QVector<QJsonObject> groups;
        QMap<QString, int> positions;
        for (const auto &value : cars_) {
            const QJsonObject car = value.toObject();
            const QString parent = car.value("parent").toString();
            const QString base = parent.isEmpty() ? car.value("id").toString() : parent;
            int position = positions.value(base, -1);
            if (position < 0) {
                QJsonObject group = car;
                group["base_id"] = base;
                group["variants"] = QJsonArray{};
                groups.append(group);
                position = groups.size() - 1;
                positions.insert(base, position);
            } else if (car.value("id").toString() == base) {
                const QJsonArray variants = groups[position].value("variants").toArray();
                const bool favorite = groups[position].value("favorite").toBool();
                const bool dashboard = groups[position].value("dashboard").toBool();
                groups[position] = car;
                groups[position]["base_id"] = base;
                groups[position]["variants"] = variants;
                groups[position]["favorite"] = favorite;
                groups[position]["dashboard"] = dashboard;
            }
            QJsonArray variants = groups[position].value("variants").toArray();
            variants.append(car);
            groups[position]["variants"] = variants;
            groups[position]["favorite"] = groups[position].value("favorite").toBool() || car.value("favorite").toBool();
            groups[position]["dashboard"] = groups[position].value("dashboard").toBool() || car.value("dashboard").toBool();
        }
        return groups;
    }

    void populateCatalog(QListWidget *list)
    {
        const bool cars = list == carList_;
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        const QString browsedChoice = catalog.layouts->currentIndex() >= 0
            ? catalog.layouts->currentData().toJsonObject().value("id").toString()
            : QString();
        QVector<QJsonObject> objects;
        if (cars) {
            objects = groupedCars();
        } else {
            objects = groupedTracks();
        }
        const QString transmission = catalog.filter ? catalog.filter->currentData().toString() : QString();
        objects.erase(std::remove_if(objects.begin(), objects.end(), [&](const QJsonObject &object) {
            if (catalog.favoritesOnly && catalog.favoritesOnly->isChecked() && !object.value("favorite").toBool()) return true;
            if (!transmission.isEmpty()) {
                bool matches = object.value("transmission").toString().split(',').contains(transmission, Qt::CaseInsensitive);
                if (cars) {
                    for (const auto &variant : object.value("variants").toArray())
                        matches = matches || variant.toObject().value("transmission").toString().split(',').contains(transmission, Qt::CaseInsensitive);
                }
                if (!matches) return true;
            }
            return false;
        }), objects.end());
        const QString ordering = catalog.sort ? catalog.sort->currentData().toString() : QStringLiteral("name");
        std::sort(objects.begin(), objects.end(), [&](const QJsonObject &left, const QJsonObject &right) {
            if (ordering == "favorite" && left.value("favorite").toBool() != right.value("favorite").toBool()) return left.value("favorite").toBool();
            if (ordering == "power") return catalogNumber(left.value("power").toString()) > catalogNumber(right.value("power").toString());
            const QString key = ordering == "author" ? QStringLiteral("author") : ordering == "power" ? QStringLiteral("power") : QStringLiteral("name");
            return left.value(key).toString().localeAwareCompare(right.value(key).toString()) < 0;
        });
        QString selectedId = catalog.selected.value("base_id").toString();
        if (selectedId.isEmpty()) {
            const QString current = cars ? selectedCar_ : selectedTrack_;
            const char *choicesKey = cars ? "variants" : "layouts";
            for (const QJsonObject &object : objects) {
                for (const auto &choice : object.value(choicesKey).toArray()) {
                    if (choice.toObject().value("id").toString() == current) {
                        selectedId = object.value("base_id").toString();
                        break;
                    }
                }
                if (!selectedId.isEmpty()) break;
            }
        }
        catalog.selected = QJsonObject();
        catalog.favorite->setEnabled(false);
        catalog.dashboard->setEnabled(false);
        catalog.use->setEnabled(false);
        list->clear();
        for (const QJsonObject &object : objects) {
            QString text;
            if (cars) {
                QStringList facts;
                if (!object.value("subtitle").toString().isEmpty()) facts.append(object.value("subtitle").toString());
                if (!object.value("transmission").toString().isEmpty()) facts.append(object.value("transmission").toString());
                if (!object.value("power").toString().isEmpty()) facts.append(object.value("power").toString());
                text = object.value("name").toString() + "\n" + facts.join(QStringLiteral(" | "));
                const int variants = object.value("variants").toArray().size();
                if (variants > 1) text += tr("\n%1 variants").arg(variants);
                if (!object.value("author").toString().isEmpty()) text += tr("\nby %1").arg(object.value("author").toString());
            } else {
                const int layouts = object.value("layouts").toArray().size();
                text = object.value("name").toString() + tr("\n%1 layout(s)").arg(layouts);
                if (!object.value("author").toString().isEmpty()) text += tr(" | by %1").arg(object.value("author").toString());
            }
            auto *item = new QListWidgetItem(QIcon(pixmapFromUrl(object.value("preview").toString(), QSize(220, 125))), text);
            item->setData(Qt::UserRole, object);
            item->setData(Qt::UserRole + 1, QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact)));
            item->setToolTip(object.value("favorite").toBool() ? tr("Favorite") : object.value("subtitle").toString());
            list->addItem(item);
            if (object.value("base_id").toString() == selectedId) { catalog.selected = object; list->setCurrentItem(item); }
        }
        if (!catalog.selected.isEmpty()) inspectCatalogItem(cars, catalog.selected, browsedChoice);
        if (cars && carCountLabel_) carCountLabel_->setText(tr("%1 models, %2 variants").arg(objects.size()).arg(cars_.size()));
        if (!cars && trackCountLabel_) trackCountLabel_->setText(tr("%1 tracks, %2 layouts").arg(groupedTracks().size()).arg(tracks_.size()));
        filterCatalogSearch(list);
    }

    void filterCatalogSearch(QListWidget *list)
    {
        const bool cars = list == carList_;
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        const QString needle = catalog.search ? catalog.search->text() : QString();
        int visible = 0;
        bool selectedHidden = false;
        for (int index = 0; index < list->count(); ++index) {
            QListWidgetItem *item = list->item(index);
            const bool hidden = !needle.isEmpty()
                && !item->data(Qt::UserRole + 1).toString().contains(needle, Qt::CaseInsensitive);
            item->setHidden(hidden);
            selectedHidden = selectedHidden || (hidden && item->isSelected());
            if (!hidden) ++visible;
        }
        if (selectedHidden) {
            list->clearSelection();
            catalog.selected = QJsonObject();
            catalog.layouts->hide();
            catalog.favorite->setEnabled(false);
            catalog.dashboard->setEnabled(false);
            catalog.use->setEnabled(false);
        }
        if (cars && carCountLabel_)
            carCountLabel_->setText(tr("%1 of %2 models, %3 variants").arg(visible).arg(list->count()).arg(cars_.size()));
        if (!cars && trackCountLabel_)
            trackCountLabel_->setText(tr("%1 of %2 tracks, %3 layouts").arg(visible).arg(list->count()).arg(tracks_.size()));
    }

    void inspectCatalogItem(bool cars, const QJsonObject &object, const QString &preferredChoice)
    {
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        catalog.selected = object;
        const QSignalBlocker blocker(catalog.layouts);
        catalog.layouts->clear();
        const QJsonArray choices = object.value(cars ? "variants" : "layouts").toArray();
        for (const auto &value : choices) {
            const QJsonObject choice = value.toObject();
            if (cars) {
                catalog.layouts->addItem(choice.value("name").toString(), choice);
            } else {
                catalog.layouts->addItem(
                    QIcon(trackPixmap(choice, QSize(144, 81))),
                    choice.value("name").toString(), choice
                );
            }
        }
        catalog.layouts->setVisible(choices.size() > 1);
        const QString current = cars ? selectedCar_ : selectedTrack_;
        int selected = -1;
        for (int index = 0; index < catalog.layouts->count(); ++index) {
            const QString id = catalog.layouts->itemData(index).toJsonObject().value("id").toString();
            if (id == preferredChoice) { selected = index; break; }
            if (selected < 0 && id == current) selected = index;
        }
        catalog.layouts->setCurrentIndex(selected >= 0 ? selected : 0);
        showCatalogDetails(cars);
    }

    void showCatalogDetails(bool cars)
    {
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        if (catalog.selected.isEmpty()) return;
        QJsonObject object = catalog.selected;
        if (catalog.layouts->currentIndex() >= 0) object = catalog.layouts->currentData().toJsonObject();
        catalog.image->setPixmap(pixmapFromUrl(object.value("preview").toString(), QSize(430, 240)));
        catalog.title->setText(object.value("name").toString());
        QStringList details;
        const QList<QPair<QString, QString>> values = cars
            ? QList<QPair<QString, QString>>{{tr("Brand"), object.value("subtitle").toString()}, {tr("Author"), object.value("author").toString()}, {tr("Class"), object.value("class_name").toString()}, {tr("Power"), object.value("power").toString()}, {tr("Torque"), object.value("torque").toString()}, {tr("Weight"), object.value("weight").toString()}, {tr("Power/weight"), object.value("power_weight").toString()}, {tr("Top speed"), object.value("top_speed").toString()}, {tr("Acceleration"), object.value("acceleration").toString()}, {tr("Drivetrain"), object.value("drivetrain").toString()}, {tr("Transmission"), object.value("transmission").toString()}}
            : QList<QPair<QString, QString>>{{tr("Author"), object.value("author").toString()}, {tr("Version"), object.value("version").toString()}, {tr("Country"), object.value("country").toString()}, {tr("City"), object.value("city").toString()}, {tr("Length"), object.value("length").toString()}, {tr("Width"), object.value("width").toString()}, {tr("Pit boxes"), object.value("pitboxes").toString()}, {tr("Direction"), object.value("direction").toString()}, {tr("Year"), object.value("year").toString()}};
        for (const auto &[label, value] : values) if (!value.isEmpty()) details.append(label + QStringLiteral(": ") + value);
        if (!object.value("description").toString().isEmpty()) details.append(QStringLiteral("\n") + object.value("description").toString());
        catalog.details->setText(details.join('\n'));
        catalog.favorite->setEnabled(true); catalog.dashboard->setEnabled(true); catalog.use->setEnabled(true);
        catalog.favorite->setText(object.value("favorite").toBool() ? tr("Remove Favorite") : tr("Add Favorite"));
        catalog.dashboard->setText(object.value("dashboard").toBool() ? tr("Remove from Dashboard") : tr("Add to Dashboard"));
    }

    void commitCatalogItem(bool cars)
    {
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        if (catalog.selected.isEmpty()) return;
        const QJsonObject choice = catalog.layouts->currentData().toJsonObject();
        if (choice.isEmpty()) return;
        if (cars) {
            selectedCar_ = choice.value("id").toString();
            selectedSkin_ = choice.value("skin").toString();
        } else {
            selectedTrack_ = choice.value("id").toString();
        }
        applyDrive();
        navigation_->setCurrentRow(0);
    }

    void toggleCatalogFavorite(bool cars)
    {
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        if (catalog.selected.isEmpty()) return;
        const QString kind = cars ? QStringLiteral("car") : QStringLiteral("track");
        const QString id = catalog.layouts->currentData().toJsonObject().value("id").toString();
        if (id.isEmpty()) return;
        QMetaObject::invokeMethod(backend_, "toggleFavorite", Q_ARG(QString, kind), Q_ARG(QString, id));
    }

    void toggleCatalogDashboard(bool cars)
    {
        CatalogWidgets &catalog = cars ? carCatalog_ : trackCatalog_;
        if (catalog.selected.isEmpty()) return;
        const QString kind = cars ? QStringLiteral("car") : QStringLiteral("track");
        const QJsonObject object = catalog.layouts->currentData().toJsonObject();
        const QString id = object.value("id").toString();
        if (!id.isEmpty()) QMetaObject::invokeMethod(backend_, "toggleDashboardItem", Q_ARG(QString, kind), Q_ARG(QString, id));
    }

    void updateDrive()
    {
        updating_ = true; mode_->setCurrentIndex(qMax(0, mode_->findData(conditions_.value("session_mode").toString("practice")))); opponents_->setValue(conditions_.value("opponents").toInt()); ai_->setValue(conditions_.value("ai_level").toInt(98)); laps_->setValue(conditions_.value("race_laps").toInt(2)); duration_->setValue(conditions_.value("session_duration").toInt()); penalties_->setChecked(conditions_.value("penalties").toBool()); weather_->setCurrentIndex(qMax(0, weather_->findData(conditions_.value("weather_id").toString()))); const double minutes = qBound(0.0, (13.0 + conditions_.value("sun_angle").toDouble() / 16.0) * 60.0, 1439.0); time_->setTime(QTime(int(minutes) / 60, int(minutes) % 60)); air_->setValue(conditions_.value("ambient_temperature").toInt(22)); road_->setValue(conditions_.value("road_temperature").toInt(28)); for (auto it = conditionEditors_.begin(); it != conditionEditors_.end(); ++it) it.value()->setValue(conditions_.value(it.key()).toDouble()); updating_ = false;
    }

    void updateSkins()
    {
        QJsonArray skins;
        for (const auto &value : cars_) {
            const auto car = value.toObject();
            if (car.value("id").toString() == selectedCar_) {
                skins = car.value("skins").toArray();
                break;
            }
        }
        bool matches = skin_->count() == skins.size();
        for (int index = 0; matches && index < skins.size(); ++index)
            matches = skin_->itemData(index).toString() == skins.at(index).toObject().value("id").toString();
        if (!matches) {
            const QSignalBlocker blocker(skin_);
            skin_->clear();
            for (const auto &value : skins) {
                const auto item = value.toObject();
                skin_->addItem(QIcon(pixmapFromUrl(item.value("preview").toString(), QSize(144, 81))),
                               item.value("name").toString(), item.value("id").toString());
            }
        }
        skin_->setCurrentIndex(qMax(0, skin_->findData(selectedSkin_)));
    }

    void updateCarVariants()
    {
        QString base;
        for (const auto &value : cars_) {
            const QJsonObject car = value.toObject();
            if (car.value("id").toString() != selectedCar_) continue;
            const QString parent = car.value("parent").toString();
            base = parent.isEmpty() ? selectedCar_ : parent;
            break;
        }
        if (base.isEmpty()) return;

        QJsonArray variants;
        for (const auto &value : cars_) {
            const QJsonObject car = value.toObject();
            if (car.value("id").toString() == base || car.value("parent").toString() == base)
                variants.append(car);
        }
        bool matches = carVariant_->count() == variants.size();
        for (int index = 0; matches && index < variants.size(); ++index)
            matches = carVariant_->itemData(index).toJsonObject().value("id").toString()
                == variants.at(index).toObject().value("id").toString();
        if (!matches) {
            const QSignalBlocker blocker(carVariant_);
            carVariant_->clear();
            for (const auto &value : variants) {
                const QJsonObject variant = value.toObject();
                carVariant_->addItem(
                    QIcon(pixmapFromUrl(variant.value("preview").toString(), QSize(144, 81))),
                    variant.value("name").toString(),
                    variant
                );
            }
        }
        int selected = -1;
        for (int index = 0; index < carVariant_->count(); ++index) {
            if (carVariant_->itemData(index).toJsonObject().value("id").toString() == selectedCar_) {
                selected = index;
                break;
            }
        }
        carVariant_->setCurrentIndex(selected >= 0 ? selected : 0);
    }

    void updateTrackLayouts()
    {
        const QString base = selectedTrack_.section('/', 0, 0);
        QJsonArray layouts;
        for (const auto &value : tracks_) {
            const QJsonObject track = value.toObject();
            if (track.value("id").toString().section('/', 0, 0) == base)
                layouts.append(track);
        }
        bool matches = trackLayout_->count() == layouts.size();
        for (int index = 0; matches && index < layouts.size(); ++index)
            matches = trackLayout_->itemData(index).toJsonObject().value("id").toString()
                == layouts.at(index).toObject().value("id").toString();
        if (!matches) {
            const QSignalBlocker blocker(trackLayout_);
            trackLayout_->clear();
            for (const auto &value : layouts) {
                const QJsonObject layout = value.toObject();
                trackLayout_->addItem(QIcon(trackPixmap(layout, QSize(144, 81))),
                                      layout.value("name").toString(), layout);
            }
        }
        int selected = -1;
        for (int i = 0; i < trackLayout_->count(); ++i) {
            if (trackLayout_->itemData(i).toJsonObject().value("id").toString() == selectedTrack_) {
                selected = i;
                break;
            }
        }
        trackLayout_->setCurrentIndex(selected >= 0 ? selected : 0);
    }

    QString selectedCarPreview() const
    {
        for (const auto &value : cars_) {
            const QJsonObject car = value.toObject();
            if (car.value("id").toString() != selectedCar_) continue;
            for (const auto &skinValue : car.value("skins").toArray()) {
                const QJsonObject skin = skinValue.toObject();
                if (skin.value("id").toString() == selectedSkin_ && !skin.value("preview").toString().isEmpty())
                    return skin.value("preview").toString();
            }
            return car.value("preview").toString();
        }
        return textProperty(backend_, "car_preview");
    }

    QString selectedTrackImage() const
    {
        for (const auto &value : tracks_) {
            const QJsonObject track = value.toObject();
            if (track.value("id").toString() != selectedTrack_) continue;
            if (!track.value("preview").toString().isEmpty()) return track.value("preview").toString();
        }
        return textProperty(backend_, "track_preview");
    }

    Backend *backend_;
    QListWidget *navigation_{}; QStackedWidget *stack_{}; QLabel *statusLabel_{}; QLabel *cspLabel_{}; QProgressBar *busy_{};
    QScrollArea *driveScroll_{}; QGridLayout *driveSelection_{}; QBoxLayout *driveQuickPanels_{}; QGridLayout *driveTimeLayout_{}; QWidget *driveCarPanel_{}; QWidget *driveTrackPanel_{}; QWidget *driveLaunchPanel_{}; QVector<QToolButton *> driveTimeButtons_; int driveLayoutMode_{-1};
    QToolButton *carPreview_{}; QToolButton *trackPreview_{}; QLabel *launchSummary_{}; QPushButton *driveButton_{}; QAction *driveAction_{}; QComboBox *mode_{}; QComboBox *carVariant_{}; QComboBox *skin_{}; QComboBox *trackLayout_{}; QComboBox *weather_{}; QSpinBox *opponents_{}; QSpinBox *ai_{}; QSpinBox *laps_{}; QSpinBox *duration_{}; QSpinBox *air_{}; QSpinBox *road_{}; QCheckBox *penalties_{}; QTimeEdit *time_{};
    QListWidget *carList_{}; QListWidget *trackList_{}; QListWidget *replayList_{}; QLineEdit *carSearch_{}; QLineEdit *trackSearch_{}; QLabel *carCountLabel_{}; QLabel *trackCountLabel_{};
    CatalogWidgets carCatalog_, trackCatalog_;
    QLabel *contentCspStatus_{}; QLineEdit *contentArchivePath_{}; QLabel *contentPreview_{}; QPushButton *contentInstall_{}; QListWidget *contentHistory_{}; QPushButton *contentRollback_{}; QLabel *contentError_{}; int contentReplacements_ = 0;
    QTableWidget *cspReleaseList_{}; QPushButton *cspInstallRelease_{};
    QLineEdit *onlineSearch_{}; QTableWidget *onlineTable_{}; QLabel *onlineDetails_{}; QComboBox *onlineCar_{}; QLineEdit *onlinePassword_{}; QPushButton *onlineJoin_{}; QLabel *onlineError_{};
    QLabel *showroomSelection_{}; QLabel *setupSummary_{}; QTableWidget *setupList_{}; QComboBox *setupTrack_{}; QLineEdit *setupName_{}; QPushButton *setupUse_{}; QPushButton *setupDelete_{}; QLabel *toolsError_{};
    QLineEdit *settingsSearch_{}; QTabBar *settingsKind_{}; QListWidget *moduleList_{}; QScrollArea *settingsScroll_{}; QLineEdit *presetName_{}; QComboBox *presetList_{};
    QTimer poll_; QTimer racePoll_; QTimer applyTimer_; bool updating_ = false;
    bool cspReleaseRequested_ = false;
    QString selectedCar_, selectedSkin_, selectedTrack_, carsCache_, tracksCache_, replayCache_, conditionsCache_, weatherCache_, acCache_, cspCache_, controlsCache_, controlInputCache_, presetsCache_, appLayoutCache_, contentPreviewCache_, contentHistoryCache_, cspReleasesCache_, serversCache_, setupsCache_;
    QJsonArray cars_, tracks_, acModules_, cspModules_, visibleModules_, onlineServers_; QJsonObject conditions_, controls_, appLayout_; QMap<QString, QDoubleSpinBox *> conditionEditors_;
    QVector<AxisMeter> axisMeters_;
    QVector<ButtonIndicator> buttonIndicators_;
};

} // namespace

int run_aclm_widgets()
{
    int argc = 1;
    char name[] = "korsa";
    char *argv[] = {name, nullptr};
    QApplication application(argc, argv);
    application.setApplicationName(QStringLiteral("Korsa"));
    application.setOrganizationName(QStringLiteral("Korsa"));
    application.setWindowIcon(QIcon(QStringLiteral(":/icons/korsa.svg")));
    QPixmapCache::setCacheLimit(128 * 1024);
    Backend backend;
    MainWindow window(&backend);
    window.show();
    return application.exec();
}
