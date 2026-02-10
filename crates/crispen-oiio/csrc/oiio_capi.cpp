#include "oiio_capi.h"

#include <OpenImageIO/imagebuf.h>
#include <OpenImageIO/imagebufalgo.h>
#include <OpenImageIO/imageio.h>

#include <algorithm>
#include <cstdlib>
#include <exception>
#include <string>

namespace OIIO = OIIO_NAMESPACE;

struct OiioImageInput
{
    OIIO::ImageBuf buf;
    std::string color_space;
};

namespace
{
thread_local std::string g_last_error;

void set_error(const char * err)
{
    g_last_error = err ? err : "unknown OIIO error";
}

void clear_error()
{
    g_last_error.clear();
}

} // namespace

// ── Error ────────────────────────────────────────────────────────────────────

extern "C" const char * oiio_get_last_error(void)
{
    return g_last_error.empty() ? nullptr : g_last_error.c_str();
}

// ── ImageInput lifecycle ─────────────────────────────────────────────────────

extern "C" OiioImageInput * oiio_image_input_open(const char * path)
{
    clear_error();
    if (!path || !path[0])
    {
        set_error("oiio_image_input_open: empty path");
        return nullptr;
    }
    try
    {
        auto h = new OiioImageInput;
        h->buf.reset(path);
        if (!h->buf.read(0, 0, false, OIIO::TypeFloat))
        {
            std::string err = h->buf.geterror();
            if (err.empty())
            {
                err = "failed to read image: " + std::string(path);
            }
            set_error(err.c_str());
            delete h;
            return nullptr;
        }
        // Cache the detected color space from the "oiio:ColorSpace" attribute.
        std::string cs = h->buf.spec()["oiio:ColorSpace"];
        if (!cs.empty())
        {
            h->color_space = std::move(cs);
        }
        return h;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" void oiio_image_input_destroy(OiioImageInput * h)
{
    delete h;
}

// ── Spec queries ─────────────────────────────────────────────────────────────

extern "C" int oiio_image_input_width(const OiioImageInput * h)
{
    return h ? h->buf.spec().width : 0;
}

extern "C" int oiio_image_input_height(const OiioImageInput * h)
{
    return h ? h->buf.spec().height : 0;
}

extern "C" int oiio_image_input_nchannels(const OiioImageInput * h)
{
    return h ? h->buf.spec().nchannels : 0;
}

extern "C" int oiio_image_input_format(const OiioImageInput * h)
{
    if (!h)
    {
        return 0;
    }
    // Return the OIIO TypeDesc BASETYPE as an int.
    return static_cast<int>(h->buf.spec().format.basetype);
}

extern "C" const char * oiio_image_input_color_space(const OiioImageInput * h)
{
    if (!h || h->color_space.empty())
    {
        return nullptr;
    }
    return h->color_space.c_str();
}

// ── Pixel reading ────────────────────────────────────────────────────────────

extern "C" int oiio_image_input_read_rgba_f32(
    const OiioImageInput * h,
    float * buf,
    int buf_len)
{
    clear_error();
    if (!h || !buf)
    {
        set_error("oiio_image_input_read_rgba_f32: null argument");
        return 0;
    }

    const int w = h->buf.spec().width;
    const int h_ = h->buf.spec().height;
    const int required = w * h_ * 4;
    if (buf_len < required)
    {
        set_error("oiio_image_input_read_rgba_f32: buffer too small");
        return 0;
    }

    try
    {
        const int nchannels = h->buf.nchannels();

        // Ensure we have exactly 4 channels (RGBA).
        OIIO::ImageBuf rgba;
        if (nchannels == 4)
        {
            rgba = h->buf;
        }
        else if (nchannels < 4)
        {
            // Map existing channels into RGBA, fill missing with 0 (alpha with 1).
            std::vector<int> order(4);
            std::vector<float> fill = {0.0f, 0.0f, 0.0f, 1.0f};
            for (int i = 0; i < 4; ++i)
            {
                order[i] = (i < nchannels) ? i : -1;
            }
            rgba = OIIO::ImageBufAlgo::channels(h->buf, 4, order, fill);
        }
        else
        {
            // More than 4 channels — take the first 4.
            std::vector<int> order = {0, 1, 2, 3};
            rgba = OIIO::ImageBufAlgo::channels(h->buf, 4, order);
        }

        if (rgba.has_error())
        {
            set_error(rgba.geterror().c_str());
            return 0;
        }

        // Use the raw-pointer get_pixels overload.
        OIIO::ROI roi = rgba.roi();
        bool ok = rgba.get_pixels(roi, OIIO::TypeFloat, buf);
        if (!ok)
        {
            std::string err = rgba.geterror();
            if (err.empty())
            {
                err = "get_pixels failed";
            }
            set_error(err.c_str());
            return 0;
        }

        return 1;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return 0;
    }
}
