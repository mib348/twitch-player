#ifndef STREAM_WIDGET_HPP
#define STREAM_WIDGET_HPP

#include <QWidget>

class VideoWidget;
class ForeignWidget;

namespace libvlc {
struct Instance;
}

class QSplitter;
class QHBoxLayout;

class StreamWidget: public QWidget {
public:
    StreamWidget(libvlc::Instance &, QWidget * = nullptr);

    void play(QString);

protected:
    void keyPressEvent(QKeyEvent *) override;

private:
    enum class ChatPosition { Left, Right };

    QSplitter *_splitter;
    QHBoxLayout *_layout;
    VideoWidget *_video;
    ForeignWidget *_chat;

    int _chat_size = 20, _video_size = 80;
    ChatPosition _chat_position = ChatPosition::Right;

    void reposition(ChatPosition);
};

#endif // STREAM_WIDGET_HPP