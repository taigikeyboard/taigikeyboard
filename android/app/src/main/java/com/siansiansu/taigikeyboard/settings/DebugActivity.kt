package com.siansiansu.taigikeyboard.settings

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.fragment.app.Fragment
import androidx.viewpager2.adapter.FragmentStateAdapter
import com.google.android.material.tabs.TabLayoutMediator
import com.siansiansu.taigikeyboard.databinding.ActivityDebugBinding
import com.siansiansu.taigikeyboard.util.setupEdgeToEdge

/**
 * Debug Zone Activity
 *
 * 顯示使用者學習資料，包括：
 * - User Frequency（詞頻）
 * - User Association（詞關聯）
 *
 * 只在 DEBUG 模式下可進入
 */
class DebugActivity : AppCompatActivity() {

    private lateinit var binding: ActivityDebugBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityDebugBinding.inflate(layoutInflater)
        setContentView(binding.root)

        setupEdgeToEdge()
        setupToolbar()
        setupViewPager()
    }

    private fun setupToolbar() {
        binding.toolbar.setNavigationOnClickListener {
            finish()
        }
    }

    private fun setupViewPager() {
        val adapter = DebugPagerAdapter(this)
        binding.viewPager.adapter = adapter

        TabLayoutMediator(binding.tabLayout, binding.viewPager) { tab, position ->
            tab.text = when (position) {
                0 -> "User Frequency"
                1 -> "User Association"
                else -> ""
            }
        }.attach()
    }

    /**
     * ViewPager Adapter
     */
    private inner class DebugPagerAdapter(activity: AppCompatActivity) : FragmentStateAdapter(activity) {
        override fun getItemCount(): Int = 2

        override fun createFragment(position: Int): Fragment {
            return when (position) {
                0 -> DebugListFragment.newInstance(DebugListFragment.TYPE_FREQUENCY)
                1 -> DebugListFragment.newInstance(DebugListFragment.TYPE_ASSOCIATION)
                else -> throw IllegalArgumentException("Invalid position: $position")
            }
        }
    }
}
