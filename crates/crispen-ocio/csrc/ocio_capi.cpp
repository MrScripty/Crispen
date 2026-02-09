#include "ocio_capi.h"

#include <OpenColorIO/OpenColorIO.h>

#include <cstdlib>
#include <exception>
#include <string>

namespace OCIO = OCIO_NAMESPACE;

struct OcioConfig
{
    OCIO::ConstConfigRcPtr config;
};

struct OcioProcessor
{
    OCIO::ConstProcessorRcPtr processor;
};

struct OcioCpuProcessor
{
    OCIO::ConstCPUProcessorRcPtr cpu;
};

namespace
{
thread_local std::string g_last_error;

const char * empty_to_null(const char * v)
{
    if (!v || !v[0])
    {
        return nullptr;
    }
    return v;
}

void set_error(const char * err)
{
    g_last_error = err ? err : "unknown OCIO error";
}

void clear_error()
{
    g_last_error.clear();
}

} // namespace

extern "C" const char * ocio_get_last_error(void)
{
    return g_last_error.empty() ? nullptr : g_last_error.c_str();
}

extern "C" OcioConfig * ocio_config_create_from_file(const char * path)
{
    clear_error();
    if (!path || !path[0])
    {
        set_error("ocio_config_create_from_file: empty path");
        return nullptr;
    }

    try
    {
        auto out = new OcioConfig;
        out->config = OCIO::Config::CreateFromFile(path);
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" OcioConfig * ocio_config_create_from_env(void)
{
    clear_error();

    const char * ocio_env = std::getenv("OCIO");
    if (!ocio_env || !ocio_env[0])
    {
        set_error("OCIO environment variable is not set");
        return nullptr;
    }

    try
    {
        auto out = new OcioConfig;
        out->config = OCIO::Config::CreateFromEnv();
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" OcioConfig * ocio_config_create_builtin(const char * uri)
{
    clear_error();
    if (!uri || !uri[0])
    {
        set_error("ocio_config_create_builtin: empty config URI");
        return nullptr;
    }

    try
    {
        auto out = new OcioConfig;
        out->config = OCIO::Config::CreateFromBuiltinConfig(uri);
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" void ocio_config_destroy(OcioConfig * config)
{
    delete config;
}

extern "C" int ocio_config_get_num_color_spaces(const OcioConfig * config)
{
    if (!config)
    {
        return -1;
    }

    try
    {
        return config->config->getNumColorSpaces();
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return -1;
    }
}

extern "C" const char * ocio_config_get_color_space_name(const OcioConfig * config, int index)
{
    if (!config)
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getColorSpaceNameByIndex(index));
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" const char * ocio_config_get_role(const OcioConfig * config, const char * role)
{
    if (!config || !role || !role[0])
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getRoleColorSpace(role));
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" int ocio_config_get_num_displays(const OcioConfig * config)
{
    if (!config)
    {
        return -1;
    }

    try
    {
        return config->config->getNumDisplays();
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return -1;
    }
}

extern "C" const char * ocio_config_get_display(const OcioConfig * config, int index)
{
    if (!config)
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getDisplay(index));
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" const char * ocio_config_get_default_display(const OcioConfig * config)
{
    if (!config)
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getDefaultDisplay());
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" int ocio_config_get_num_views(const OcioConfig * config, const char * display)
{
    if (!config || !display || !display[0])
    {
        return -1;
    }

    try
    {
        return config->config->getNumViews(display);
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return -1;
    }
}

extern "C" const char * ocio_config_get_view(
    const OcioConfig * config,
    const char * display,
    int index
)
{
    if (!config || !display || !display[0])
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getView(display, index));
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" const char * ocio_config_get_default_view(
    const OcioConfig * config,
    const char * display
)
{
    if (!config || !display || !display[0])
    {
        return nullptr;
    }

    try
    {
        return empty_to_null(config->config->getDefaultView(display));
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" OcioProcessor * ocio_config_get_processor_by_names(
    const OcioConfig * config,
    const char * src,
    const char * dst
)
{
    clear_error();
    if (!config || !src || !src[0] || !dst || !dst[0])
    {
        set_error("ocio_config_get_processor_by_names: invalid args");
        return nullptr;
    }

    try
    {
        auto out = new OcioProcessor;
        out->processor = config->config->getProcessor(src, dst);
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" OcioProcessor * ocio_config_get_display_view_processor(
    const OcioConfig * config,
    const char * src,
    const char * display,
    const char * view
)
{
    clear_error();
    if (!config || !src || !src[0] || !display || !display[0] || !view || !view[0])
    {
        set_error("ocio_config_get_display_view_processor: invalid args");
        return nullptr;
    }

    try
    {
        auto out = new OcioProcessor;
        out->processor = config->config->getProcessor(src, display, view, OCIO::TRANSFORM_DIR_FORWARD);
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" void ocio_processor_destroy(OcioProcessor * proc)
{
    delete proc;
}

extern "C" OcioCpuProcessor * ocio_processor_get_cpu_f32(const OcioProcessor * proc)
{
    clear_error();
    if (!proc)
    {
        set_error("ocio_processor_get_cpu_f32: null processor");
        return nullptr;
    }

    try
    {
        auto out = new OcioCpuProcessor;
        out->cpu = proc->processor->getDefaultCPUProcessor();
        return out;
    }
    catch (const OCIO::Exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return nullptr;
    }
}

extern "C" void ocio_cpu_processor_destroy(OcioCpuProcessor * cpu)
{
    delete cpu;
}

extern "C" void ocio_cpu_processor_apply_rgba(
    const OcioCpuProcessor * cpu,
    float * pixels,
    int width,
    int height
)
{
    if (!cpu || !pixels || width <= 0 || height <= 0)
    {
        return;
    }

    try
    {
        OCIO::PackedImageDesc img(
            pixels,
            width,
            height,
            4,
            OCIO::BIT_DEPTH_F32,
            static_cast<std::ptrdiff_t>(sizeof(float)),
            static_cast<std::ptrdiff_t>(4 * sizeof(float)),
            static_cast<std::ptrdiff_t>(width) * 4 * static_cast<std::ptrdiff_t>(sizeof(float))
        );
        cpu->cpu->apply(img);
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
    }
}

extern "C" void ocio_cpu_processor_apply_rgb_pixel(const OcioCpuProcessor * cpu, float * pixel)
{
    if (!cpu || !pixel)
    {
        return;
    }

    try
    {
        cpu->cpu->applyRGB(pixel);
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
    }
}

extern "C" int ocio_cpu_processor_is_noop(const OcioCpuProcessor * cpu)
{
    if (!cpu)
    {
        return 1;
    }

    try
    {
        return cpu->cpu->isNoOp() ? 1 : 0;
    }
    catch (const std::exception & e)
    {
        set_error(e.what());
        return 1;
    }
}
