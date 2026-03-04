package com.siansiansu.taigikeyboard.settings

import android.app.AlertDialog
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Toast
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.DividerItemDecoration
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.databinding.FragmentDebugListBinding
import com.siansiansu.taigikeyboard.databinding.ItemDebugEntryBinding
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Debug List Fragment
 *
 * 顯示 User Frequency 或 User Association 資料
 */
class DebugListFragment : Fragment() {

    private var _binding: FragmentDebugListBinding? = null
    private val binding get() = _binding!!

    private var listType: Int = TYPE_FREQUENCY
    private var allData: List<DebugEntry> = emptyList()
    private var filteredData: List<DebugEntry> = emptyList()
    private lateinit var adapter: DebugAdapter

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        listType = arguments?.getInt(ARG_TYPE, TYPE_FREQUENCY) ?: TYPE_FREQUENCY
    }

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        _binding = FragmentDebugListBinding.inflate(inflater, container, false)
        return binding.root
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        setupRecyclerView()
        setupSearch()
        setupClearButton()
        loadData()
    }

    private fun setupRecyclerView() {
        adapter = DebugAdapter()
        binding.recyclerView.apply {
            layoutManager = LinearLayoutManager(requireContext())
            adapter = this@DebugListFragment.adapter
            addItemDecoration(DividerItemDecoration(requireContext(), DividerItemDecoration.VERTICAL))
        }
    }

    private fun setupSearch() {
        binding.searchInput.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: Editable?) {
                filterData(s?.toString() ?: "")
            }
        })
    }

    private fun setupClearButton() {
        binding.btnClear.setOnClickListener {
            AlertDialog.Builder(requireContext())
                .setTitle("Clear All Data")
                .setMessage("Are you sure you want to clear all ${if (listType == TYPE_FREQUENCY) "frequency" else "association"} data?")
                .setPositiveButton("Clear") { _, _ ->
                    clearData()
                }
                .setNegativeButton("Cancel", null)
                .show()
        }
    }

    private fun loadData() {
        lifecycleScope.launch {
            val data = withContext(Dispatchers.IO) {
                when (listType) {
                    TYPE_FREQUENCY -> loadFrequencyData()
                    TYPE_ASSOCIATION -> loadAssociationData()
                    else -> emptyList()
                }
            }

            allData = data
            filteredData = data
            updateUI()
        }
    }

    private suspend fun loadFrequencyData(): List<DebugEntry> {
        return UserFrequencyService.getAllFrequencies(requireContext()).map { (word, count) ->
            DebugEntry(
                primary = word,
                secondary = null,
                count = count
            )
        }
    }

    private suspend fun loadAssociationData(): List<DebugEntry> {
        return NextWordService.allAssociations(requireContext()).map { association ->
            DebugEntry(
                primary = "${association.prevWord} → ${association.nextWord}",
                secondary = if (association.nextTl.isNotEmpty()) {
                    "TL: ${association.nextTl}"
                } else null,
                count = association.count
            )
        }
    }

    private fun filterData(query: String) {
        filteredData = if (query.isEmpty()) {
            allData
        } else {
            allData.filter { entry ->
                entry.primary.contains(query, ignoreCase = true) ||
                    (entry.secondary?.contains(query, ignoreCase = true) == true)
            }
        }
        updateUI()
    }

    private fun updateUI() {
        adapter.submitList(filteredData)

        binding.statsCount.text = "Total: ${filteredData.size} / ${allData.size} entries"

        if (filteredData.isEmpty()) {
            binding.recyclerView.visibility = View.GONE
            binding.emptyText.visibility = View.VISIBLE
        } else {
            binding.recyclerView.visibility = View.VISIBLE
            binding.emptyText.visibility = View.GONE
        }
    }

    private fun clearData() {
        lifecycleScope.launch {
            withContext(Dispatchers.IO) {
                when (listType) {
                    TYPE_FREQUENCY -> UserFrequencyService.clearAllFrequencies(requireContext())
                    TYPE_ASSOCIATION -> NextWordService.clearAllAssociations(requireContext())
                }
            }

            Toast.makeText(requireContext(), "Data cleared", Toast.LENGTH_SHORT).show()
            loadData()
        }
    }

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null
    }

    /**
     * Debug Entry Data Class
     */
    data class DebugEntry(
        val primary: String,
        val secondary: String?,
        val count: Int
    )

    /**
     * RecyclerView Adapter
     */
    private inner class DebugAdapter : RecyclerView.Adapter<DebugAdapter.ViewHolder>() {
        private var items: List<DebugEntry> = emptyList()

        fun submitList(newItems: List<DebugEntry>) {
            items = newItems
            notifyDataSetChanged()
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val binding = ItemDebugEntryBinding.inflate(
                LayoutInflater.from(parent.context), parent, false
            )
            return ViewHolder(binding)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            holder.bind(items[position])
        }

        override fun getItemCount(): Int = items.size

        inner class ViewHolder(private val binding: ItemDebugEntryBinding) :
            RecyclerView.ViewHolder(binding.root) {

            fun bind(entry: DebugEntry) {
                binding.textPrimary.text = entry.primary
                binding.textCount.text = entry.count.toString()

                if (entry.secondary != null) {
                    binding.textSecondary.visibility = View.VISIBLE
                    binding.textSecondary.text = entry.secondary
                } else {
                    binding.textSecondary.visibility = View.GONE
                }
            }
        }
    }

    companion object {
        const val TYPE_FREQUENCY = 0
        const val TYPE_ASSOCIATION = 1

        private const val ARG_TYPE = "type"

        fun newInstance(type: Int): DebugListFragment {
            return DebugListFragment().apply {
                arguments = Bundle().apply {
                    putInt(ARG_TYPE, type)
                }
            }
        }
    }
}
