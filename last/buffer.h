typedef struct Line {
    char *line;
    int len;
} Line;

typedef struct Buffer {
    Line *contents[];
    int len;
} Buffer;

int buf_init(Buffer *buf)
{
    buf.contents = {};
    buf.len = 0;
}


