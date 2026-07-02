#include <linux/init.h>
#include <linux/module.h>

MODULE_LICENSE("GPL");

static int __init ferrum_init(void)
{
    pr_info("Ferrum driver loaded\n");
    return 0;
}

static void __exit ferrum_exit(void)
{
    pr_info("Ferrum driver unloaded\n");
}

module_init(ferrum_init);
module_exit(ferrum_exit);
